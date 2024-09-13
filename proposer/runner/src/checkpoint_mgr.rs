use crate::proposer::ServerState;
use bincode::Options;
use common::ordinary_clock::{Clock, OrdinaryClock};
use dag::DagLedger;
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, HashMap},
    sync::Arc,
};
use tee_vlc::nitro_clock::NitroEnclavesClock;
use tokio::{net::UdpSocket, sync::RwLock};
use tracing::*;
use types::configuration::{Round, GENESIS_ROUND};

#[derive(Debug, Clone, Default)]
pub struct CheckpointMgr {
    pub state: Arc<RwLock<ServerState>>,
    pub dag: DagLedger, // persist to db
    pub round: Round,
    pub cache_round_maximum: u32, // move to config
    pub clocks_in_round: BTreeMap<Round, HashMap<String, (NitroEnclavesClock, Vec<String>)>>,
    pub last_transaction_index: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CheckpointMessage {
    pub addr: String,
    pub round: Round,
    pub clock: NitroEnclavesClock,
    pub tx_ids: Vec<String>,
}

impl CheckpointMgr {
    pub fn new(state: Arc<RwLock<ServerState>>, dag: DagLedger) -> Self {
        Self {
            round: GENESIS_ROUND,
            clocks_in_round: BTreeMap::new(),
            last_transaction_index: 0,
            dag,
            state,
            cache_round_maximum: 32,
        }
    }

    pub fn _from(
        state: Arc<RwLock<ServerState>>,
        round: Round,
        clocks_in_round: BTreeMap<Round, HashMap<String, (NitroEnclavesClock, Vec<String>)>>,
        last_transaction_index: u64,
        dag: DagLedger,
    ) -> Self {
        Self {
            state,
            round,
            clocks_in_round,
            last_transaction_index,
            dag,
            cache_round_maximum: 32,
        }
    }

    pub fn insert_round(
        &mut self,
        round: Round,
        key: String,
        value: (NitroEnclavesClock, Vec<String>),
    ) {
        if self.clocks_in_round.len() >= self.cache_round_maximum as usize {
            if let Some((&min_key, _)) = self.clocks_in_round.iter().next() {
                self.clocks_in_round.remove(&min_key);
            }
        }

        self.clocks_in_round
            .entry(round)
            .or_insert(HashMap::new())
            .insert(key, value);
    }

    pub fn _get_round(
        &self,
        round: Round,
    ) -> Option<&HashMap<String, (NitroEnclavesClock, Vec<String>)>> {
        self.clocks_in_round.get(&round)
    }

    pub async fn new_round(
        &mut self,
        addr: &String,
        peers: &Vec<String>,
        this_clock: &mut NitroEnclavesClock,
        new_clock: &OrdinaryClock,
        p2p_socket: &UdpSocket,
        tx_ids: Vec<String>
    ) {
        if new_clock.partial_cmp(&this_clock.plain) == Some(Ordering::Less) {
            debug!(
                "Old stale, new_clock {:?} is less than current clock {:?}",
                new_clock, this_clock
            );
            return;
        }

        info!(
            "start new round checkpoint, round: {}, local clock {:?}, new clock {:?}",
            self.round, this_clock, new_clock
        );

        this_clock.plain = new_clock.clone();

        self.insert_round(
            self.round,
            addr.to_string(),
            (this_clock.clone(), tx_ids.to_vec()),
        );

        // broadcast, sprawn??
        let msg = CheckpointMessage {
            addr: addr.to_string(),
            round: self.round,
            clock: this_clock.clone(),
            tx_ids,
        };

        trace!("start broadcast, msg = {:?}", msg);
        let msg_bytes = bincode::options().serialize(&msg).unwrap();
        for addr_i in peers {
            p2p_socket.send_to(&msg_bytes, addr_i).await.unwrap();
        }
    }

    pub async fn try_commit_checkpoint(&mut self, msg: CheckpointMessage, peers: &Vec<String>) {
        let tx_ids;
        {
            // immutable reference, read mutex guard
            let msg_ids = &self.state.read().await.message_ids;
            if !msg_ids.is_empty() {
                tx_ids = vec![msg_ids.last().unwrap().to_string()];
            } else {
                if self.round == 0 {
                    tx_ids = vec![];
                } else {
                    info!("message_ids is empty, exist checkpoint");
                    return; // or handle the case where msg_ids is empty
                }
            }
        }
        // mutable reference, &mut self, and &self can't be in same contex
        self.insert_round(msg.round, msg.addr, (msg.clock, msg.tx_ids));
        self.commit_checkpoint(peers, &tx_ids).await;
    }

    async fn commit_checkpoint(&mut self, peers: &Vec<String>, tx_ids: &Vec<String>) {
        let this_clock = &mut self.state.write().await.trusted_vlc;
        debug!(
            "Commit checkpoint clocks_in_round is {:?}",
            self.clocks_in_round
        );
        for (round, votes) in self.clocks_in_round.iter() {
            let committee_size = peers.len() + 1;
            if votes.keys().len() == peers.len() + 1 {
                // todo: consider if is needs to remove

                // Check if the count of valid entries exceeds 2/3 + 1 of total entries
                if votes.keys().len() > (committee_size * 2) / 3 + 1 {
                    // Merge state & gen consensus clock
                    info!("Merging state and committing checkpoint to DAG");
                    let valid_clock: Vec<OrdinaryClock> = votes
                        .iter()
                        .map(|(_addr, (clock, _))| clock.plain.clone())
                        .collect();
                    // Todo: round diff more, how to do next?
                    let consensus_clock = OrdinaryClock::base(valid_clock.iter());
                    let new_tx_nums = consensus_clock.reduce();

                    if self.last_transaction_index < new_tx_nums {
                        // commit checkpoint to DAG and inc round
                        let height = round + 1;
                        for (_addr, value) in votes.iter() {
                            info!(
                                "DAG: add consensus node, height {}, value {:?}",
                                height, value
                            );
                            self.dag.add_node(
                                height,
                                &value.1,
                                value.0.clone(),
                                self.dag.last_height_parents.clone(),
                            );
                        }

                        // update checkpoint state
                        this_clock.plain = consensus_clock;
                        self.last_transaction_index = new_tx_nums;
                        self.round = self.round + 1;

                        let height = self.round + 1;
                        info!(
                            "DAG: add checkpoint node, height {}, value {:?}",
                            height, this_clock
                        );
                        self.dag.add_node(
                            height,
                            tx_ids,
                            this_clock.clone(),
                            self.dag.last_height_parents.clone(),
                        );
                    } else {
                        warn!("Cannot advance checkpoint: last tx index bigger than the new consensus clock");
                    }
                } else {
                    error!(
                        "Cannot advance checkpoint: not enough valid entries, current {}, need {}",
                        votes.len(),
                        (committee_size * 2) / 3 + 1
                    );
                }
            }
        }
        // Clean invalidated messages by removing all keys less than the current round
        self.clocks_in_round.retain(|&k, _| k >= self.round);
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;
    use tokio::time::timeout;

    use super::*;

    #[tokio::test]
    #[ignore = "loop func"]
    async fn test_commit_checkpoint() {
        let mut checkpoint_state = CheckpointMgr {
            round: 1,
            clocks_in_round: BTreeMap::new(),
            last_transaction_index: 0,
            ..Default::default()
        };

        let tx_ids = vec!["tx1".to_string()];
        checkpoint_state
            .clocks_in_round
            .entry(1)
            .or_default()
            .insert(
                "self".to_string(),
                (NitroEnclavesClock::default(), tx_ids.clone()),
            );
        checkpoint_state
            .clocks_in_round
            .entry(1)
            .or_default()
            .insert(
                "peer2".to_string(),
                (NitroEnclavesClock::default(), tx_ids.clone()),
            );
        checkpoint_state
            .clocks_in_round
            .entry(1)
            .or_default()
            .insert(
                "peer3".to_string(),
                (NitroEnclavesClock::default(), tx_ids.clone()),
            );
        checkpoint_state
            .clocks_in_round
            .entry(2)
            .or_default()
            .insert(
                "peer3".to_string(),
                (NitroEnclavesClock::default(), tx_ids.clone()),
            );

        let peers = vec![
            "peer1".to_string(),
            "peer2".to_string(),
            "peer3".to_string(),
        ];

        let timeout_duration = Duration::from_secs(3);
        let _ = timeout(
            timeout_duration,
            checkpoint_state.commit_checkpoint(&peers, &vec![]),
        )
        .await;
    }
}
