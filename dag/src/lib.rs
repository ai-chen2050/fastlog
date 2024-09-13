use common::crypto::core::DigestHash;
use fastlog_core::messages::CertifiedTransferOrder;
use std::collections::{HashMap, HashSet};
use tee_vlc::nitro_clock::NitroEnclavesClock;
use tracing::*;

/// Define a struct for a certified transaction
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct _CertifiedTransaction {
    id: String, // blake2 hash function for generating
    timestamp: u64,
    certified_tx: CertifiedTransferOrder,
}

// Define a struct for a DAG node
#[derive(Debug, Clone)]
pub struct DagNode {
    pub height: u64, // can remove after upgrading to dual-clock
    pub verified_clock: NitroEnclavesClock,
    pub parents: HashSet<String>,
    pub tx_id_batch: Vec<String>,
}

// Define a struct for the DAG ledger
#[derive(Debug, Clone, Default)]
pub struct DagLedger {
    pub total_height: u64,
    pub last_height_parents: HashSet<String>,
    nodes: HashMap<String, DagNode>, // clock_hash as the key
}

impl DagLedger {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            total_height: 0,
            last_height_parents: HashSet::new(),
        }
    }

    pub fn add_node(
        &mut self,
        height: u64,
        tx_id_batch: &Vec<String>,
        verified_clock: NitroEnclavesClock,
        parents: HashSet<String>,
    ) {
        debug!(
            "Add dag node: input height {}, total_height {}",
            height, self.total_height
        );
        use DigestHash as _;
        let node_id = verified_clock.plain.blake2().to_string();
        let node = DagNode {
            verified_clock,
            parents,
            height,
            tx_id_batch: tx_id_batch.to_vec(),
        };
        if height == self.total_height + 1 {
            self.nodes.insert(node_id.clone(), node);
            self.total_height = height;
            self.last_height_parents.insert(node_id);
        } else if height == self.total_height {
            self.nodes.insert(node_id.clone(), node);
        }
    }

    pub fn get_dag_node(&self, id: &str) -> Option<&DagNode> {
        self.nodes.get(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_dag_ledger() {
        let ledger = DagLedger::new();
        assert_eq!(ledger.total_height, 0);
        assert!(ledger.nodes.is_empty());
    }

    #[test]
    fn test_add_node() {
        let mut ledger = DagLedger::new();

        let height = 1;
        let tx_id_batch = vec!["tx1".to_string(), "tx2".to_string()];
        let verified_clock = NitroEnclavesClock::default();
        let parents: HashSet<String> = HashSet::new();

        ledger.add_node(
            height,
            &tx_id_batch,
            verified_clock.clone(),
            parents.clone(),
        );

        assert_eq!(ledger.total_height, height);

        use DigestHash as _;
        let key = verified_clock.plain.blake2().to_string();
        assert!(ledger.nodes.contains_key(&key));

        let node = ledger.get_dag_node(&key).unwrap();
        assert_eq!(node.height, height);
        assert_eq!(node.verified_clock, verified_clock);
        assert_eq!(node.parents, parents);
        assert_eq!(node.tx_id_batch, tx_id_batch);
    }

    #[test]
    fn test_add_node_with_same_height() {
        let mut ledger = DagLedger::new();

        let height = 1;
        let tx_id_batch1 = vec!["tx1".to_string(), "tx2".to_string()];
        let verified_clock1 = NitroEnclavesClock::default();
        verified_clock1.plain.update(Vec::new().iter(), 0);
        let parents1: HashSet<String> = HashSet::new();
        use DigestHash as _;
        let key1 = verified_clock1.plain.blake2().to_string();

        ledger.add_node(
            height,
            &tx_id_batch1,
            verified_clock1.clone(),
            parents1.clone(),
        );

        let tx_id_batch2 = vec!["tx3".to_string(), "tx4".to_string()];
        let verified_clock2 = NitroEnclavesClock::default();
        verified_clock2.plain.update(Vec::new().iter(), 1);
        let parents2: HashSet<String> = HashSet::new();
        let key2 = verified_clock2.plain.blake2().to_string();

        ledger.add_node(
            height,
            &tx_id_batch2,
            verified_clock2.clone(),
            parents2.clone(),
        );

        // Ensure the node with the same height is not added
        assert!(ledger.nodes.contains_key(&key1));
        assert!(ledger.nodes.contains_key(&key2));
        assert!(ledger.total_height == 1);

        let tx_id_batch3 = vec!["tx3".to_string(), "tx4".to_string()];
        let verified_clock3 = NitroEnclavesClock::default();
        verified_clock3.plain.update(Vec::new().iter(), 3);
        let parents3: HashSet<String> = HashSet::new();
        let key3 = verified_clock3.plain.blake2().to_string();

        ledger.add_node(
            height + 1,
            &tx_id_batch3,
            verified_clock3.clone(),
            parents3.clone(),
        );

        assert!(ledger.nodes.contains_key(&key3));
        assert!(ledger.total_height == 2);
    }
}