use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Represents the type of a transaction
#[derive(Debug, Deserialize, PartialEq, Clone)]
pub enum TransactionType {
    #[serde(rename = "deposit")]
    Deposit,
    #[serde(rename = "withdrawal")]
    Withdrawal,
    #[serde(rename = "dispute")]
    Dispute,
    #[serde(rename = "resolve")]
    Resolve,
    #[serde(rename = "chargeback")]
    Chargeback,
}

/// Represents a transaction record from the input CSV
#[derive(Debug, Deserialize, Clone)]
pub struct Transaction {
    #[serde(rename = "type")]
    pub tx_type: TransactionType,
    pub client: u16,
    #[serde(rename = "tx")]
    pub id: u32,
    pub amount: Option<f64>,
}

/// Represents a client record, which is updated by transactions
#[derive(Debug, Serialize)]
pub struct Client {
    pub client: u16,
    pub available: f64,
    pub held: f64,
    pub total: f64,
    pub locked: bool,
}

impl Client {
    pub fn new(client: u16) -> Self {
        Self {
            client,
            available: 0.0,
            held: 0.0,
            total: 0.0,
            locked: false,
        }
    }
}

/// A simple implentation of a first-in-last-out buffer
/// with a fixed capacity, which will drop the oldest item
/// when a new item exceeds the capacity.
pub struct RingBuffer<T> {
    inside: VecDeque<T>,
}

impl<T> RingBuffer<T> {
    ///Create a new `RingBuffer` with a capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inside: VecDeque::with_capacity(capacity),
        }
    }
    ///Push a new item into the buffer, removing the oldest
    ///item if the buffer is full
    pub fn push(&mut self, item: T) {
        if self.inside.len() == self.inside.capacity() {
            self.inside.pop_front();
        }
        self.inside.push_back(item);
    }

    ///Pop the oldest item from the buffer
    ///Only needed in tests
    #[cfg(test)]
    pub fn pop(&mut self) -> Option<T> {
        self.inside.pop_front()
    }

    ///Returns whether the buffer is empty
    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.inside.is_empty()
    }
}

impl RingBuffer<Transaction> {
    ///Get a transaction by its ID from the buffer
    ///
    ///There may be more efficient ways to search for a transaction by ID, but
    ///since disputes should be rarer than deposits and withdrawals, it makes
    ///most sense to primarily optimize a buffer for adding and removing transactions
    pub fn get_by_tx(&self, id: u32) -> Option<&Transaction> {
        self.inside.iter().find(|tx| tx.id == id)
    }
}
