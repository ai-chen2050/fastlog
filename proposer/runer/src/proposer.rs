use crate::{node_factory::ProposerFactory, storage::Storage};
use alloy_primitives::B256;
use alloy_wrapper::contracts::vrf_range::OperatorRangeContract;
use node_api::config::ProposerConfig;
use std::collections::VecDeque;
use std::sync::Arc;
use tee_vlc::nitro_clock::{NitroEnclavesClock, Update};
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::RwLock;
// use tracing::*;

pub struct Proposer {
    pub config: Arc<ProposerConfig>,
    pub _storage: Storage,
    pub _state: RwLock<ServerState>,
    pub _tee_vlc_sender: UnboundedSender<Update<NitroEnclavesClock>>,
    pub _vrf_range_contract: OperatorRangeContract,
}

pub type ProposerArc = Arc<Proposer>;

impl Proposer {
    pub fn proposer_factory() -> ProposerFactory {
        ProposerFactory::init()
    }

    pub fn _update_tee_sender(mut self, sender: UnboundedSender<Update<NitroEnclavesClock>>) {
        self._tee_vlc_sender = sender;
    }
}

/// A cache state of a server node.
#[derive(Debug, Clone)]
pub struct ServerState {
    // pub clock_info: ClockInfo,
    pub _signer_key: B256,
    pub _message_ids: VecDeque<String>,
    pub _cache_maximum: u64,
}

impl ServerState {
    /// Create a new server state.
    pub fn new(signer: B256, _node_id: String, cache_maximum: u64) -> Self {
        Self {
            _signer_key: signer,
            _message_ids: VecDeque::new(),
            _cache_maximum: cache_maximum,
        }
    }
}
