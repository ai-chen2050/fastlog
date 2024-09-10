use bincode::Options;
use common::ordinary_clock::OrdinaryClock;
use serde::{Deserialize, Serialize};
use tokio::net::UdpSocket;
use std::{cmp::Ordering, collections::HashMap};
use tee_vlc::nitro_clock::NitroEnclavesClock;
use types::configuration::{Round, GENESIS_ROUND};

#[derive(Debug, Clone, Default)]
pub struct CheckpointState {
    pub round: Round,
    pub clocks_in_round: HashMap<String, (Round, NitroEnclavesClock)>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CheckpointMessage {
    pub addr: String,
    pub round: Round,
    pub clocks: NitroEnclavesClock,
}

impl CheckpointState {
    pub fn _new() -> Self {
        Self {
            round: GENESIS_ROUND,
            clocks_in_round: HashMap::new(),
        }
    }

    pub fn _from(
        round: Round,
        clocks_in_round: HashMap<String, (Round, NitroEnclavesClock)>,
    ) -> Self {
        Self {
            round,
            clocks_in_round,
        }
    }

    pub async fn new_round(
        &mut self,
        addr: &String,
        peers: &Vec<String>,
        this_clock: &mut NitroEnclavesClock,
        new_clock: &OrdinaryClock,
        p2p_socket: &UdpSocket,
    ) {
        self.clocks_in_round = HashMap::new();
        if new_clock.partial_cmp(&this_clock.plain) == Some(Ordering::Greater) {
            this_clock.plain = new_clock.clone();
        }
        self.clocks_in_round
            .insert(addr.to_string(), (self.round, this_clock.clone()));

        // broadcast, sprawn?? 
        let msg = CheckpointMessage {
            addr: addr.to_string(),
            round: self.round,
            clocks: this_clock.clone(),
        };
        let msg_bytes = bincode::options().serialize(&msg).unwrap();
        for addr_i in peers {
            p2p_socket
                .send_to(
                    &msg_bytes,
                    addr_i,
                )
                .await
                .unwrap();
        }

        self.commit_checkpoint(peers);
    }

    fn commit_checkpoint(&mut self, peers: &Vec<String>) -> ! {
        loop {
            if self.clocks_in_round.keys().len() == peers.len() + 1 {
                // merge state & commit checkpoint to dag
            }
        }
    }
}
