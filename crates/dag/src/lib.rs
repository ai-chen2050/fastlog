use std::collections::{HashMap, HashSet};

// Define a struct for a transaction
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Transaction {
    id: String,
    timestamp: u64,
    data: String,
}

// Define a struct for a vector clock
#[derive(Debug, Clone)]
struct VectorClock {
    timestamps: HashMap<String, u64>,
}

impl VectorClock {
    fn new() -> Self {
        Self {
            timestamps: HashMap::new(),
        }
    }

    fn update(&mut self, node_id: &str, timestamp: u64) {
        self.timestamps.insert(node_id.to_string(), timestamp);
    }

    fn merge(&mut self, other: &VectorClock) {
        for (node_id, timestamp) in &other.timestamps {
            let entry = self.timestamps.entry(node_id.clone()).or_insert(*timestamp);
            *entry = (*entry).max(*timestamp);
        }
    }
}

// Define a struct for a DAG node
#[derive(Debug, Clone)]
struct DagNode {
    transaction: Transaction,
    vector_clock: VectorClock,
    parents: HashSet<String>,
}

// Define a struct for the DAG ledger
struct DagLedger {
    nodes: HashMap<String, DagNode>,
}

impl DagLedger {
    fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    fn add_transaction(&mut self, transaction: Transaction, vector_clock: VectorClock, parents: HashSet<String>) {
        let node = DagNode {
            transaction: transaction.clone(),
            vector_clock,
            parents,
        };
        self.nodes.insert(transaction.id.clone(), node);
    }

    fn update_vector_clock(&mut self, node_id: &str, clock: &VectorClock) {
        if let Some(node) = self.nodes.get_mut(node_id) {
            node.vector_clock.merge(clock);
        }
    }

    fn get_transaction(&self, id: &str) -> Option<&Transaction> {
        self.nodes.get(id).map(|node| &node.transaction)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ledger() {
        let mut ledger = DagLedger::new();

        // Example transaction and vector clock
        let tx1 = Transaction {
            id: "tx1".to_string(),
            timestamp: 1,
            data: "Transaction 1".to_string(),
        };

        let mut vc1 = VectorClock::new();
        vc1.update("node1", 1);

        ledger.add_transaction(tx1, vc1.clone(), HashSet::new());

        // Add more transactions and update vector clocks as needed
        let tx2 = Transaction {
            id: "tx2".to_string(),
            timestamp: 1,
            data: "Transaction 2".to_string(),
        };

        vc1.update("node2", 1);

        ledger.add_transaction(tx2, vc1.clone(), HashSet::new());

        // Fetch a transaction
        if let Some(transaction) = ledger.get_transaction("tx1") {
            println!("{:?}", transaction);
        }

        if let Some(transaction) = ledger.get_transaction("tx2") {
            println!("{:?}", transaction);
        }
    }
}
