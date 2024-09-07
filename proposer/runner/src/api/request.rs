// use alloy_primitives::hex::FromHex;
// use alloy_primitives::B256;
// use alloy_wrapper::util::sign_message;
// use common::crypto::core::DigestHash;
use crate::api::response::WorkerStatus;
use node_api::config::ProposerConfig;
use node_api::error::ProposerError;
use reqwest::Client as ReqwestClient;
use serde::{Deserialize, Serialize};
use tee_vlc::nitro_clock::{NitroEnclavesClock, UpdateOk};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::time::{sleep, Duration};
use tools::helper::machine_used;
use tracing::{debug, error, info};

#[derive(serde::Serialize)]
pub struct RegisterWorkerReq {
    pub worker_name: String,
    pub check_heart_beat: bool,
    pub worker_status: WorkerStatus,
    pub multimodal: bool,
}

#[derive(serde::Serialize)]
pub struct RegisterHeartbeatReq {
    pub worker_name: String,
    pub node_id: String,
    pub queue_length: u32,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct InferParams {
    pub temperature: f32,
    pub top_p: f32,
    pub max_tokens: u32,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct QuestionReq {
    pub request_id: String,
    pub node_id: String,
    pub model: String,
    pub prompt: String,
    pub params: InferParams,
    pub prompt_hash: String,
    pub signature: String,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct AnswerCallbackReq {
    request_id: String,
    node_id: String,
    model: String,
    prompt: String,
    answer: String,
    elapsed: u64,
    selected: bool,
    vrf_proof: VRFProof,
    tee_credential: TEECredential,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct VRFProof {
    pub vrf_prompt_hash: String,
    pub vrf_random_value: String,
    pub vrf_verify_pubkey: String,
    pub vrf_proof: String,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct TEECredential {
    pub tee_attestation: String,
    pub tee_attest_signature: String,
}

#[derive(Debug, Deserialize, Serialize, Default)]
struct HeartbeatResp {
    exist: bool,
}

pub async fn register_worker(config: &ProposerConfig) -> Result<reqwest::Response, reqwest::Error> {
    let (cpu_percent, cpu_nums, memory_total, memory_used) = machine_used();
    let worker_status = WorkerStatus {
        node_id: config.node.node_id.clone(),
        model_names: config.node.ai_models.clone(),
        cpu_percent: format!("{:.2}%", cpu_percent),
        cpu_nums: format!("{} cores", cpu_nums),
        mem_total: format!("{} M", memory_total / 1024 / 1024),
        mem_used: format!("{} M", memory_used / 1024 / 1024),
        speed: 1,
        queue_length: 0,
    };

    let body = RegisterWorkerReq {
        worker_name: config.net.outer_url.clone(),
        check_heart_beat: true,
        worker_status,
        multimodal: false,
    };

    let client = ReqwestClient::new();
    client
        .post(format!(
            "{}{}",
            config.net.dispatcher_url.clone(),
            "/register_worker"
        ))
        .header("Content-Type", "application/json; charset=utf-8")
        .json(&body)
        .send()
        .await
}

async fn register_heartbeat(config: &ProposerConfig) -> Result<reqwest::Response, reqwest::Error> {
    debug!("Registering heartbeat to dispatcher...");

    let body = RegisterHeartbeatReq {
        worker_name: config.net.outer_url.clone(),
        node_id: config.node.node_id.clone(),
        queue_length: 0,
    };

    let client = ReqwestClient::new();
    client
        .post(format!(
            "{}{}",
            config.net.dispatcher_url.clone(),
            "/receive_heart_beat"
        ))
        .header("Content-Type", "application/json; charset=utf-8")
        .json(&body)
        .send()
        .await
}

pub async fn periodic_heartbeat_task(config: ProposerConfig) {
    let interval = Duration::from_secs(config.node.heartbeat_interval);
    loop {
        match register_heartbeat(&config).await {
            Ok(response) => {
                debug!("Response status: {}", response.status());
                match response.text().await {
                    Ok(body) => {
                        debug!("Response body: {}", body);
                        let json = serde_json::from_str(&body).unwrap_or_default();
                        let data: HeartbeatResp = serde_json::from_value(json).unwrap_or_default();
                        if !data.exist {
                            let response = register_worker(&config)
                                .await
                                .map_err(ProposerError::OPSetupRegister)
                                .unwrap();
                            if response.status().is_success() {
                                info!(
                                    "register worker to dispatcher success! response_body: {:?}",
                                    response.text().await
                                )
                            }
                        }
                    }
                    Err(err) => error!("Failed to read response body, {}", err),
                }
            }
            Err(err) => error!("periodic heartbeat request error, {}", err),
        }
        sleep(interval).await;
    }
}

pub async fn listening_tee_resp_task(
    _config: ProposerConfig,
    mut receiver: UnboundedReceiver<UpdateOk<NitroEnclavesClock>>,
) {
    loop {
        if let Some(resp) = receiver.recv().await {
            debug!(
                "Response id: {}, clock: {:?}, metric: {:?}",
                resp.0, resp.1, resp.2
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::{Client, Error};

    #[ignore = "local api"]
    #[tokio::test]
    async fn register() -> Result<(), Error> {
        let body = RegisterWorkerReq {
            worker_name: "http://localhost:21002".to_string(),
            check_heart_beat: true,
            worker_status: WorkerStatus {
                model_names: vec!["vicuna-7b-v1.5".to_string()],
                speed: 1,
                queue_length: 0,
                node_id: "todo!()".to_string(),
                cpu_percent: "todo!()".to_string(),
                cpu_nums: "todo!()".to_string(),
                mem_total: "todo!()".to_string(),
                mem_used: "todo!()".to_string(),
            },
            multimodal: false,
        };

        let client = Client::new();
        let response = client
            .post("http://127.0.0.1:21001/register_worker")
            .header("Content-Type", "application/json; charset=utf-8")
            .json(&body)
            .send()
            .await?;

        println!("Response status: {}", response.status());
        let response_body = response.text().await?;
        println!("Response body: {}", response_body);

        Ok(())
    }
}
