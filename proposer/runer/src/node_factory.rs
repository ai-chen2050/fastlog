use crate::api::read::not_found;
use crate::api::request::{listening_tee_resp_task, periodic_heartbeat_task, register_worker};
use crate::handler::router;
use crate::proposer::{Proposer, ProposerArc, ServerState};
use crate::storage;
use actix_web::{middleware, web, App, HttpServer};
use alloy_primitives::hex::FromHex;
use alloy_primitives::B256;
use alloy_wrapper::contracts::vrf_range::new_vrf_range_backend;
use node_api::config::ProposerConfig;
use node_api::error::ProposerError;
use node_api::error::{
    ProposerError::{OPDecodeSignerKeyError, OPNewVrfRangeContractError},
    ProposerResult,
};
use std::sync::Arc;
use tee_vlc::nitro_clock::{
    tee_start_listening, try_connection, NitroEnclavesClock, Update, UpdateOk,
};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::sync::RwLock;
use tracing::info;

#[derive(Default)]
pub struct ProposerFactory {
    pub config: ProposerConfig,
}

impl ProposerFactory {
    pub fn init() -> Self {
        Self::default()
    }

    pub fn set_config(mut self, config: ProposerConfig) -> Self {
        self.config = config;
        self
    }

    pub async fn create_proposer(
        config: ProposerConfig,
        tee_inference_sender: UnboundedSender<Update<NitroEnclavesClock>>,
    ) -> ProposerResult<ProposerArc> {
        let cfg = Arc::new(config.clone());
        let node_id = config.node.node_id.clone();
        let signer_key =
            B256::from_hex(config.node.signer_key.clone()).map_err(OPDecodeSignerKeyError)?;
        let vrf_range_contract = new_vrf_range_backend(
            &config.chain.chain_rpc_url,
            &config.chain.vrf_range_contract,
        )
        .map_err(OPNewVrfRangeContractError)?;

        let server_state = ServerState::new(signer_key, node_id, cfg.node.cache_msg_maximum);
        let state = RwLock::new(server_state);
        let storage = storage::Storage::new(cfg.clone()).await;
        let proposer = Proposer {
            config: cfg,
            _storage: storage,
            _state: state,
            _tee_vlc_sender: tee_inference_sender,
            _vrf_range_contract: vrf_range_contract,
        };

        Ok(Arc::new(proposer))
    }

    async fn create_actix_node(arc_proposer: ProposerArc) {
        let arc_proposer_clone = Arc::clone(&arc_proposer);

        let app = move || {
            App::new()
                .app_data(web::Data::new(arc_proposer_clone.clone()))
                .wrap(middleware::Logger::default()) // enable logger
                .default_service(web::route().to(not_found))
                .configure(router)
        };

        HttpServer::new(app)
            .bind(arc_proposer.config.net.rest_url.clone())
            .expect("Failed to bind address")
            .run()
            .await
            .expect("Failed to run server");
    }

    async fn prepare_setup(
        config: &ProposerConfig,
    ) -> ProposerResult<UnboundedSender<Update<NitroEnclavesClock>>> {
        // detect and connect tee enclave service, if not, and exit
        let (prompt_sender, prompt_receiver) = unbounded_channel::<Update<NitroEnclavesClock>>();
        let (answer_ok_sender, answer_ok_receiver) =
            unbounded_channel::<UpdateOk<NitroEnclavesClock>>();

        let (tee_cid, tee_port) = (config.net.tee_vlc_cid, config.net.tee_vlc_port);
        let result = try_connection(tee_cid, tee_port);
        if let Err(err) = result {
            return Err(ProposerError::OPConnectTEEError(err.to_string()));
        } else {
            info!("connect llm tee service successed!");
        }

        tokio::spawn(tee_start_listening(
            result.unwrap(),
            prompt_receiver,
            answer_ok_sender,
        ));

        // register status to dispatcher service
        let response = register_worker(config)
            .await
            .map_err(ProposerError::OPSetupRegister)?;

        if response.status().is_success() {
            info!(
                "register worker to dispatcher success! response_body: {:?}",
                response.text().await
            )
        } else {
            return Err(ProposerError::CustomError(format!(
                "Error: register to dispatcher failed, resp code {}",
                response.status()
            )));
        }

        // periodic heartbeat task
        let config_clone = config.clone();
        tokio::spawn(periodic_heartbeat_task(config_clone));

        // answer callback
        let config_clone = config.clone();
        tokio::spawn(listening_tee_resp_task(config_clone, answer_ok_receiver));

        Ok(prompt_sender)
    }

    pub async fn initialize_node(self) -> ProposerResult<ProposerArc> {
        let prompt_sender = ProposerFactory::prepare_setup(&self.config).await?;

        let arc_proposer =
            ProposerFactory::create_proposer(self.config.clone(), prompt_sender).await?;

        ProposerFactory::create_actix_node(arc_proposer.clone()).await;

        Ok(arc_proposer)
    }
}
