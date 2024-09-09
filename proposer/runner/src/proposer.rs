use crate::{node_factory::ProposerFactory, storage::Storage};
use alloy::hex::ToHexExt;
use alloy_primitives::B256;
use common::crypto::core::DigestHash;
use fastlog_core::messages::CertifiedTransferOrder;
use fastlog_core::serialize::deserialize_message;
use node_api::config::ProposerConfig;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tee_vlc::nitro_clock::{NitroEnclavesClock, Update, UpdateOk};
use tokio::net::UdpSocket;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};
use tracing::*;
use types::configuration::DEFAULT_MAX_DATAGRAM_SIZE;

pub struct Proposer {
    pub config: Arc<ProposerConfig>,
    pub _storage: Storage,
    pub state: RwLock<ServerState>,
    pub _tee_vlc_sender: UnboundedSender<Update<NitroEnclavesClock>>,
    pub txs_commit_socket: UdpSocket,
}

pub type ProposerArc = Arc<Proposer>;

impl Proposer {
    pub fn proposer_factory() -> ProposerFactory {
        ProposerFactory::init()
    }

    pub async fn periodic_proposal(self: Arc<Self>) {
        let interval = Duration::from_millis(self.config.node.proposal_interval_ms);
        loop {
            todo!("more detail")
        }
        sleep(interval).await;
    }

    pub async fn handle_certified_commit(self: Arc<Self>) {
        info!(
            "Now worker of certified commit listen on : {}",
            self.config.net.txs_commit_udp
        );
        loop {
            let mut buf = [0; DEFAULT_MAX_DATAGRAM_SIZE];
            let (n, _src) = self.txs_commit_socket.recv_from(&mut buf).await.unwrap();
            let msg = deserialize_message(&buf[..n]);
            if let Ok(sm) = msg {
                match sm {
                    fastlog_core::serialize::SerializedMessage::Cert(certified_tx) => {
                        use DigestHash as _;
                        let tx_id = certified_tx.value.transfer.blake2().encode_hex();
                        self.state.write().await.cache_commited_txs.insert(tx_id, *certified_tx);
                    }
                    _ => {
                        error!("Unsuported fastlog_core message");
                    }
                };
            } else {
                error!("Err msg or invalid serialize raw bytes");
            }
        }
    }

    pub async fn listening_tee_resp_task(
        self: Arc<Self>,
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

    pub fn _update_tee_sender(mut self, sender: UnboundedSender<Update<NitroEnclavesClock>>) {
        self._tee_vlc_sender = sender;
    }
}

/// A cache state of a server node.
#[derive(Debug, Clone)]
pub struct ServerState {
    pub trusted_vlc: NitroEnclavesClock,
    pub _signer_key: B256,
    pub _message_ids: VecDeque<String>,
    pub cache_commited_txs: HashMap<String, CertifiedTransferOrder>,
    pub _cache_maximum: u64,
}

impl ServerState {
    /// Create a new server state.
    pub fn new(signer: B256, _node_id: String, cache_maximum: u64) -> Self {
        Self {
            trusted_vlc: NitroEnclavesClock::default(),
            _signer_key: signer,
            _message_ids: VecDeque::new(),
            cache_commited_txs: HashMap::new(),
            _cache_maximum: cache_maximum,
        }
    }
}
