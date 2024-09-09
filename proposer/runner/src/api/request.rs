use crate::api::response::ProposerStatus;
use serde::{Deserialize, Serialize};

#[derive(serde::Serialize)]
pub struct RegisterWorkerReq {
    pub worker_name: String,
    pub check_heart_beat: bool,
    pub worker_status: ProposerStatus,
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


#[cfg(test)]
mod tests {

    #[ignore = "local api"]
    #[tokio::test]
    async fn out() {
        println!("request out");
    }
}
