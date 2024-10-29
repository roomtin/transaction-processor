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
    client: u16,
    pub available: f64,
    pub held: f64,
    pub total: f64,
    pub locked: bool,
}

impl Client {
    pub fn new(client: u16) -> Client {
        Client {
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
    pub fn with_capacity(capacity: usize) -> RingBuffer<T> {
        RingBuffer {
            inside: VecDeque::with_capacity(capacity),
        }
    }
    pub fn push(&mut self, item: T) {
        if self.inside.len() == self.inside.capacity() {
            self.inside.pop_front();
        }
        self.inside.push_back(item);
    }

    #[allow(dead_code)]
    pub fn pop(&mut self) -> Option<T> {
        self.inside.pop_front()
    }
}

/// Implement a method to get a transaction by its ID from the ring buffer
impl RingBuffer<Transaction> {
    pub fn get_by_tx(&self, id: u32) -> Option<&Transaction> {
        self.inside.iter().find(|tx| tx.id == id)
    }
}

#[test]
fn test_ring_buffer() {
    let mut buffer: RingBuffer<u32> = RingBuffer::with_capacity(5);
    buffer.push(1);
    buffer.push(2);
    buffer.push(3);
    buffer.push(4);
    buffer.push(5);
    buffer.push(6);
    assert_eq!(buffer.pop(), Some(2));
    assert_eq!(buffer.pop(), Some(3));
    assert_eq!(buffer.pop(), Some(4));
    assert_eq!(buffer.pop(), Some(5));
    assert_eq!(buffer.pop(), Some(6));
    assert_eq!(buffer.pop(), None);
}
