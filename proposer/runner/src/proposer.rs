use crate::{node_factory::ProposerFactory, storage::Storage};
use alloy::hex::ToHexExt;
use alloy_primitives::B256;
use common::crypto::core::DigestHash;
use fastlog::config::AuthorityServerConfig;
use fastlog_core::base_types::PublicKeyBytes;
use fastlog_core::messages::{CertifiedTransferOrder, PullStateClockRequest};
use fastlog_core::serialize::{deserialize_message, serialize_pull_state_request};
use node_api::config::ProposerConfig;
use node_api::error::ProposerError::PROClientBindUDPError;
use std::collections::{HashMap, VecDeque};
use std::net::SocketAddr;
use std::sync::Arc;
use tee_vlc::nitro_clock::{NitroEnclavesClock, Update, UpdateOk};
use tokio::net::UdpSocket;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};
use tracing::*;
use types::configuration::DEFAULT_MAX_DATAGRAM_SIZE;

pub struct Proposer {
    pub pro_config: Arc<ProposerConfig>,
    pub auth_config: Arc<AuthorityServerConfig>,
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
        let interval = Duration::from_millis(self.pro_config.node.proposal_interval_ms);
        let num_shards = self.auth_config.authority.num_shards;
        let base_port = self.auth_config.authority.base_port;
        let (tx, rx) = mpsc::channel(num_shards as usize);
        let socket = Arc::new(
            UdpSocket::bind("127.0.0.1:0")
                .await
                .map_err(PROClientBindUDPError)
                .unwrap(),
        );
        let addresses: Arc<Vec<SocketAddr>> = Arc::new(
            (0..num_shards)
                .map(|i| format!("127.0.0.1:{}", base_port + i).parse().unwrap())
                .collect(),
        );

        self.clone().start_checkpoints(rx).await;
        self.pull_shard_states(addresses, tx, socket, interval)
            .await
    }

    async fn start_checkpoints(self: Arc<Self>, mut rx: mpsc::Receiver<(u32, u64)>) {
        tokio::spawn(async move {
            while let Some((index, value)) = rx.recv().await {
                println!("Received result: index={}, value={}", index, value);
            }
        });
    }

    async fn pull_shard_states(
        self: Arc<Self>,
        addresses: Arc<Vec<SocketAddr>>,
        tx: mpsc::Sender<(u32, u64)>,
        socket: Arc<UdpSocket>,
        interval: Duration,
    ) -> ! {
        loop {
            for (index, addr) in addresses.iter().enumerate() {
                let tx = tx.clone();
                let socket = socket.clone();
                let addr = addr.clone();
                let self_clone = self.clone();
                tokio::spawn(async move {
                    let req = PullStateClockRequest {
                        sender: PublicKeyBytes([1; 32]),
                        shard_id: index as u32,
                    };
                    let buf = serialize_pull_state_request(&req);
                    socket.send_to(&buf, &addr).await.expect("send error");

                    let mut buf = [0; DEFAULT_MAX_DATAGRAM_SIZE];
                    let (_len, _addr) = socket.recv_from(&mut buf).await.expect("recv error");
                    let (index, value) = self_clone.parse_shard_state_resp(&buf);
                    tx.send((index, value))
                        .await
                        .expect("pull state send to channel failed");
                });
            }
            sleep(interval).await;
        }
    }

    fn parse_shard_state_resp(self: Arc<Self>, buf: &[u8]) -> (u32, u64) {
        let result = deserialize_message(buf).unwrap();
        match result {
            fastlog_core::serialize::SerializedMessage::PullStateResp(resp) => {
                (resp.shard_id, resp.total_counts.into())
            }
            _ => {
                println!("not support");
                (0, 0)
            }
        }
    }

    pub async fn handle_certified_commit(self: Arc<Self>) {
        info!(
            "Now worker of certified commit listen on : {}",
            self.pro_config.net.txs_commit_udp
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
                        self.state
                            .write()
                            .await
                            .cache_commited_txs
                            .insert(tx_id, *certified_tx);
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
