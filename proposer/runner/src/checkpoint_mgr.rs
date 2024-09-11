use bincode::Options;
use common::ordinary_clock::{Clock, OrdinaryClock};
use dag::DagLedger;
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, collections::HashMap};
use tee_vlc::nitro_clock::NitroEnclavesClock;
use tokio::net::UdpSocket;
use tracing::*;
use types::configuration::{Round, GENESIS_ROUND};

#[derive(Debug, Clone, Default)]
pub struct CheckpointState {
    pub round: Round,
    pub clocks_in_round: HashMap<String, (Round, NitroEnclavesClock, Vec<String>)>,
    pub last_transaction_index: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CheckpointMessage {
    pub addr: String,
    pub round: Round,
    pub clock: NitroEnclavesClock,
    pub tx_ids: Vec<String>,
}

impl CheckpointState {
    pub fn _new() -> Self {
        Self {
            round: GENESIS_ROUND,
            clocks_in_round: HashMap::new(),
            last_transaction_index: 0,
        }
    }

    pub fn _from(
        round: Round,
        clocks_in_round: HashMap<String, (Round, NitroEnclavesClock, Vec<String>)>,
        last_transaction_index: u64,
    ) -> Self {
        Self {
            round,
            clocks_in_round,
            last_transaction_index,
        }
    }

    pub async fn new_round(
        &mut self,
        addr: &String,
        peers: &Vec<String>,
        this_clock: &mut NitroEnclavesClock,
        new_clock: &OrdinaryClock,
        p2p_socket: &UdpSocket,
        dag: &mut DagLedger,
        tx_ids: &Vec<String>,
    ) {
        self.clocks_in_round = HashMap::new();
        if new_clock.partial_cmp(&this_clock.plain) == Some(Ordering::Greater) {
            this_clock.plain = new_clock.clone();
        }
        self.clocks_in_round.insert(
            addr.to_string(),
            (self.round, this_clock.clone(), tx_ids.to_vec()),
        );

        // broadcast, sprawn??
        let msg = CheckpointMessage {
            addr: addr.to_string(),
            round: self.round,
            clock: this_clock.clone(),
            tx_ids: tx_ids.to_vec(),
        };
        let msg_bytes = bincode::options().serialize(&msg).unwrap();
        for addr_i in peers {
            p2p_socket.send_to(&msg_bytes, addr_i).await.unwrap();
        }

        self.commit_checkpoint(peers, this_clock, dag, tx_ids).await;
    }

    async fn commit_checkpoint(
        &mut self,
        peers: &Vec<String>,
        this_clock: &mut NitroEnclavesClock,
        dag: &mut DagLedger,
        tx_ids: &Vec<String>,
    ) {
        loop {
            if self.clocks_in_round.keys().len() == peers.len() + 1 {
                let valid_entries: Vec<_> = self
                    .clocks_in_round
                    .iter()
                    .filter(|&(_, &(round, _, _))| round == self.round)
                    .collect();

                // Check if the count of valid entries exceeds 2/3 + 1 of total entries
                if valid_entries.len() > (self.clocks_in_round.len() * 2) / 3 + 1 {
                    // Merge state & gen consensus clock
                    info!("Merging state and committing checkpoint to DAG");
                    let valid_clock: Vec<OrdinaryClock> = valid_entries
                        .iter()
                        .map(|(_addr, (_round, clock, _))| clock.plain.clone())
                        .collect();
                    let consensus_clock = OrdinaryClock::base(valid_clock.iter());
                    // commit checkpoint to DAG and inc round
                    let height = dag.total_height + 1;
                    for (_addr, value) in self.clocks_in_round.iter() {
                        dag.add_node(height, &value.2, value.1.clone(), dag.last_height_parents.clone());
                    }

                    // update checkpoint state
                    let new_tx_nums = consensus_clock.reduce();
                    if self.last_transaction_index < new_tx_nums {
                        this_clock.plain = consensus_clock;
                        self.last_transaction_index = new_tx_nums;
                        self.round = self.round + 1;
                        
                        let height = dag.total_height + 1;
                        dag.add_node(height, tx_ids, this_clock.clone(), dag.last_height_parents.clone());
                    } else {
                        error!("Cannot advance checkpoint: last tx index bigger than the new consensus clock");
                    }
                } else {
                    error!("Cannot advance checkpoint: not enough valid entries");
                }
            }
        }
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
        let mut checkpoint_state = CheckpointState {
            round: 1,
            clocks_in_round: HashMap::new(),
            last_transaction_index: 0,
        };

        let tx_ids = Vec::from(["tx1".to_string()]);
        // Add some entries to clocks_in_round for testing
        checkpoint_state.clocks_in_round.insert(
            "self".to_string(),
            (1, NitroEnclavesClock::default(), tx_ids.clone()),
        );
        checkpoint_state.clocks_in_round.insert(
            "peer2".to_string(),
            (1, NitroEnclavesClock::default(), tx_ids.clone()),
        );
        checkpoint_state.clocks_in_round.insert(
            "peer3".to_string(),
            (1, NitroEnclavesClock::default(), tx_ids.clone()),
        );
        checkpoint_state.clocks_in_round.insert(
            "peer3".to_string(),
            (2, NitroEnclavesClock::default(), tx_ids),
        );

        let peers = vec![
            "peer1".to_string(),
            "peer2".to_string(),
            "peer3".to_string(),
        ];

        let timeout_duration = Duration::from_secs(3);
        // Call commit_checkpoint with a timeout
        let _ = timeout(
            timeout_duration,
            checkpoint_state.commit_checkpoint(
                &peers,
                &mut NitroEnclavesClock::default(),
                &mut DagLedger::new(),
                &Vec::new(),
            ),
        )
        .await;
    }
}
