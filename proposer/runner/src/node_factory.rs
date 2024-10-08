use crate::api::read::not_found;
use crate::handler::router;
use crate::proposer::{Proposer, ProposerArc, ServerState};
use crate::{checkpoint_mgr::CheckpointMgr, storage};
use actix_web::{middleware, web, App, HttpServer};
use alloy_primitives::hex::FromHex;
use alloy_primitives::B256;
use dag::DagLedger;
use fastlog::config::AuthorityServerConfig;
use node_api::config::ProposerConfig;
use node_api::error::ProposerError;
use node_api::error::{
    ProposerError::{PRODecodeSignerKeyError, PROServerBindUDPError},
    ProposerResult,
};
use std::sync::Arc;
use tee_vlc::nitro_clock::{
    tee_start_listening, try_connection, NitroEnclavesClock, Update, UpdateOk,
};
use tokio::net::UdpSocket;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::sync::RwLock;
use tracing::info;

#[derive(Default)]
pub struct ProposerFactory {
    pub pro_config: ProposerConfig,
    pub auth_config: AuthorityServerConfig,
}

impl ProposerFactory {
    pub fn init() -> Self {
        Self::default()
    }

    pub fn set_config(mut self, config: ProposerConfig, auth: AuthorityServerConfig) -> Self {
        self.pro_config = config;
        self.auth_config = auth;
        self
    }

    pub async fn create_proposer(
        pro_config: ProposerConfig,
        auth_config: AuthorityServerConfig,
        tee_vlc_sender: UnboundedSender<Update<NitroEnclavesClock>>,
        tee_vlc_receiver: UnboundedReceiver<UpdateOk<NitroEnclavesClock>>,
    ) -> ProposerResult<ProposerArc> {
        let pro_cfg = Arc::new(pro_config.clone());
        let auth_cfg = Arc::new(auth_config);
        let node_id = pro_config.node.node_id.clone();
        let signer_key =
            B256::from_hex(pro_config.node.signer_key.clone()).map_err(PRODecodeSignerKeyError)?;
        let txs_commit_socket = UdpSocket::bind(pro_config.net.txs_commit_udp)
            .await
            .map_err(|err| {
                PROServerBindUDPError(format!("txs_commit_socket, errmsg {}", err.to_string()))
            })?;
        let p2p_socket = UdpSocket::bind(pro_config.net.p2p.listen_address)
            .await
            .map_err(|err| {
                PROServerBindUDPError(format!("p2p_socket, errmsg {}", err.to_string()))
            })?;
        let server_state = ServerState::new(signer_key, node_id, pro_cfg.node.cache_msg_maximum);
        let state = Arc::new(RwLock::new(server_state));
        let storage = storage::Storage::new(pro_cfg.clone()).await;

        let checkpoint_mgr = Arc::new(RwLock::new(CheckpointMgr::new(
            state.clone(),
            DagLedger::new(),
        )));
        let proposer = Proposer {
            pro_config: pro_cfg,
            auth_config: auth_cfg,
            _storage: storage,
            state,
            _tee_vlc_sender: tee_vlc_sender,
            txs_commit_socket,
            p2p_socket,
            checkpoint_mgr,
        };
        let arc_proposer = Arc::new(proposer);

        let arc_proposer_clone = arc_proposer.clone();
        tokio::spawn(arc_proposer_clone.handle_certified_commit());
        tokio::spawn(arc_proposer.clone().handle_p2p_message());
        tokio::spawn(arc_proposer.clone().periodic_proposal());
        tokio::spawn(arc_proposer.clone().listening_tee_resp(tee_vlc_receiver));

        Ok(arc_proposer)
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
            .bind(arc_proposer.pro_config.net.rest_url.clone())
            .expect("Failed to bind address")
            .run()
            .await
            .expect("Failed to run server");
    }

    async fn prepare_setup(
        config: &ProposerConfig,
    ) -> ProposerResult<(
        UnboundedSender<Update<NitroEnclavesClock>>,
        UnboundedReceiver<UpdateOk<NitroEnclavesClock>>,
    )> {
        // detect and connect tee enclave service, if not, and exit
        let (vlc_tee_sender, vlc_reply_receiver) =
            unbounded_channel::<Update<NitroEnclavesClock>>();
        let (answer_ok_sender, answer_ok_receiver) =
            unbounded_channel::<UpdateOk<NitroEnclavesClock>>();

        let (tee_cid, tee_port) = (config.net.tee_vlc_cid, config.net.tee_vlc_port);
        let result = try_connection(tee_cid, tee_port);
        if let Err(err) = result {
            return Err(ProposerError::PROConnectTEEError(err.to_string()));
        } else {
            info!("connect llm tee service successed!");
        }

        tokio::spawn(tee_start_listening(
            result.unwrap(),
            vlc_reply_receiver,
            answer_ok_sender,
        ));

        Ok((vlc_tee_sender, answer_ok_receiver))
    }

    pub async fn initialize_node(self) -> ProposerResult<ProposerArc> {
        // skip tee dectect
        // let (vlc_tee_tx, vlc_tee_rx) = ProposerFactory::prepare_setup(&self.pro_config).await?;

        let (vlc_tee_tx, _vlc_reply_receiver) = unbounded_channel::<Update<NitroEnclavesClock>>();
        let (_answer_ok_sender, vlc_tee_rx) = unbounded_channel::<UpdateOk<NitroEnclavesClock>>();

        let arc_proposer = ProposerFactory::create_proposer(
            self.pro_config,
            self.auth_config,
            vlc_tee_tx,
            vlc_tee_rx,
        )
        .await?;

        ProposerFactory::create_actix_node(arc_proposer.clone()).await;

        Ok(arc_proposer)
    }
}
