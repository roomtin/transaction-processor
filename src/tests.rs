use crate::datatypes::{RingBuffer, Transaction, TransactionType};
use crate::process_transaction;
use std::collections::HashMap;

///RingBuffer should allow pushing as many items as its capacity
///and popping them in the order they were pushed, dropping the oldest
///item when the buffer is full
#[test]
fn test_ring_buffer() {
    let mut buffer: RingBuffer<u32> = RingBuffer::with_capacity(3);

    buffer.push(1);
    buffer.push(2);
    buffer.push(3);
    buffer.push(4);
    assert_eq!(buffer.pop(), Some(2));
    assert_eq!(buffer.pop(), Some(3));
    assert_eq!(buffer.pop(), Some(4));
    assert_eq!(buffer.pop(), None);
}

///Test the get_by_tx function
#[test]
fn test_get_by_tx() {
    let mut clients = HashMap::new();
    let mut processed_txs = RingBuffer::with_capacity(10);
    let mut held_txs = HashMap::new();

    let transactions = (1..=20).map(|i| Transaction {
        tx_type: TransactionType::Deposit,
        client: 1,
        id: i,
        amount: Some(i as f64),
    });

    for tx in transactions {
        process_transaction(tx, &mut clients, &mut processed_txs, &mut held_txs).unwrap();
    }

    let tx = processed_txs.get_by_tx(18).unwrap();
    assert_eq!(tx.amount.unwrap(), 18.0);
}

///Test that deposits behave correctly
///
///Deposits should increase the client's available and total funds
#[test]
fn test_deposit() {
    let mut clients = HashMap::new();
    let mut processed_txs = RingBuffer::with_capacity(10);
    let mut held_txs = HashMap::new();

    let tx = Transaction {
        tx_type: TransactionType::Deposit,
        client: 1,
        id: 1,
        amount: Some(20.1234),
    };

    process_transaction(tx, &mut clients, &mut processed_txs, &mut held_txs).unwrap();

    let client = clients.get(&1).unwrap();
    assert_eq!(client.available, 20.1234);
    assert_eq!(client.total, 20.1234);
}

///Test that withdrawals behave correctly
///
///Withdrawals should decrease the client's available and total funds, but should not
///be allowed if the client has insufficient available funds
#[test]
fn test_withdrawal() {
    let mut clients = HashMap::new();
    let mut processed_txs = RingBuffer::with_capacity(10);
    let mut held_txs = HashMap::new();

    let tx = Transaction {
        tx_type: TransactionType::Deposit,
        client: 1,
        id: 1,
        amount: Some(20.1234),
    };

    process_transaction(tx, &mut clients, &mut processed_txs, &mut held_txs).unwrap();

    let client = clients.get(&1).unwrap();
    assert_eq!(client.available, 20.1234);
    assert_eq!(client.total, 20.1234);

    let tx = Transaction {
        tx_type: TransactionType::Withdrawal,
        client: 1,
        id: 2,
        amount: Some(10.1234),
    };

    process_transaction(tx, &mut clients, &mut processed_txs, &mut held_txs).unwrap();

    let client = clients.get(&1).unwrap();
    assert_eq!(client.available, 10.0);
    assert_eq!(client.total, 10.0);

    let tx = Transaction {
        tx_type: TransactionType::Withdrawal,
        client: 1,
        id: 3,
        amount: Some(20.0),
    };

    let result = process_transaction(tx, &mut clients, &mut processed_txs, &mut held_txs);

    //assert that the withdrawal fails and the client's funds are unchanged
    let client = clients.get(&1).unwrap();
    assert_eq!(client.available, 10.0);
    assert_eq!(client.total, 10.0);
    assert_eq!(result.is_err(), true);
}

///Test that disputes behave correctly
///
///Disputes should move the disputed transaction's amount from available to held funds
#[test]
fn test_dispute() {
    let mut clients = HashMap::new();
    let mut processed_txs = RingBuffer::with_capacity(10);
    let mut held_txs = HashMap::new();

    let tx = Transaction {
        tx_type: TransactionType::Deposit,
        client: 1,
        id: 1,
        amount: Some(20.1234),
    };

    process_transaction(tx, &mut clients, &mut processed_txs, &mut held_txs).unwrap();

    let tx = Transaction {
        tx_type: TransactionType::Deposit,
        client: 1,
        id: 2,
        amount: Some(10.0),
    };

    process_transaction(tx, &mut clients, &mut processed_txs, &mut held_txs).unwrap();

    let tx = Transaction {
        tx_type: TransactionType::Dispute,
        client: 1,
        id: 2,
        amount: None,
    };

    process_transaction(tx, &mut clients, &mut processed_txs, &mut held_txs).unwrap();

    //The client's available funds should be decreased by the amount of the disputed transaction
    //and the held funds should be increased by the amount of the disputed transaction
    //and the total funds should be unchanged
    let client = clients.get(&1).unwrap();
    assert_eq!(client.available, 20.1234);
    assert_eq!(client.held, 10.0);
    assert_eq!(client.total, 30.1234);

    //The disputed transaction should be in the held_txs hashmap
    assert_eq!(held_txs.contains_key(&2), true);
}

///Test that resolves behave correctly
///
///Resolves should move the disputed transaction's amount from held to available funds
///and remove the transaction from the held_txs hashmap
#[test]
fn test_resolve() {
    let mut clients = HashMap::new();
    let mut processed_txs = RingBuffer::with_capacity(10);
    let mut held_txs = HashMap::new();

    let tx = Transaction {
        tx_type: TransactionType::Deposit,
        client: 1,
        id: 1,
        amount: Some(20.1234),
    };

    process_transaction(tx, &mut clients, &mut processed_txs, &mut held_txs).unwrap();

    let tx = Transaction {
        tx_type: TransactionType::Deposit,
        client: 1,
        id: 2,
        amount: Some(10.0),
    };

    process_transaction(tx, &mut clients, &mut processed_txs, &mut held_txs).unwrap();

    let tx = Transaction {
        tx_type: TransactionType::Dispute,
        client: 1,
        id: 2,
        amount: None,
    };

    process_transaction(tx, &mut clients, &mut processed_txs, &mut held_txs).unwrap();

    let tx = Transaction {
        tx_type: TransactionType::Resolve,
        client: 1,
        id: 2,
        amount: None,
    };

    process_transaction(tx, &mut clients, &mut processed_txs, &mut held_txs).unwrap();

    //The client's available funds should be increased by the amount of the disputed transaction
    //and the held funds should be decreased by the amount of the disputed transaction
    //and the total funds should be unchanged
    let client = clients.get(&1).unwrap();
    assert_eq!(client.available, 30.1234);
    assert_eq!(client.held, 0.0);
    assert_eq!(client.total, 30.1234);

    //The disputed transaction should be removed from the held_txs hashmap
    assert_eq!(held_txs.contains_key(&2), false);
}

///Test that chargebacks behave correctly
///
///Chargebacks should withdraw the disputed transaction's amount from the
///client's held and total funds
#[test]
fn test_chargeback() {
    let mut clients = HashMap::new();
    let mut processed_txs = RingBuffer::with_capacity(10);
    let mut held_txs = HashMap::new();

    let tx = Transaction {
        tx_type: TransactionType::Deposit,
        client: 1,
        id: 1,
        amount: Some(20.1234),
    };

    process_transaction(tx, &mut clients, &mut processed_txs, &mut held_txs).unwrap();

    let tx = Transaction {
        tx_type: TransactionType::Deposit,
        client: 1,
        id: 2,
        amount: Some(10.0),
    };

    process_transaction(tx, &mut clients, &mut processed_txs, &mut held_txs).unwrap();

    let tx = Transaction {
        tx_type: TransactionType::Dispute,
        client: 1,
        id: 2,
        amount: None,
    };

    process_transaction(tx, &mut clients, &mut processed_txs, &mut held_txs).unwrap();

    let tx = Transaction {
        tx_type: TransactionType::Chargeback,
        client: 1,
        id: 2,
        amount: None,
    };

    process_transaction(tx, &mut clients, &mut processed_txs, &mut held_txs).unwrap();

    //The client's held funds should be decreased by the amount of the disputed transaction
    //and the total funds should be decreased by the amount of the disputed transaction
    //and the available funds should be unchanged
    //and the client should be locked after a chargeback
    let client = clients.get(&1).unwrap();
    assert_eq!(client.available, 20.1234);
    assert_eq!(client.held, 0.0);
    assert_eq!(client.total, 20.1234);
    assert_eq!(client.locked, true);

    //The disputed transaction should be removed from the held_txs hashmap
    assert_eq!(held_txs.contains_key(&2), false);
}

///Test that client amounts are rounded to 4 decimal places
#[test]
fn test_rounding() {
    let mut clients = HashMap::new();
    let mut processed_txs = RingBuffer::with_capacity(10);
    let mut held_txs = HashMap::new();

    let tx = Transaction {
        tx_type: TransactionType::Deposit,
        client: 1,
        id: 1,
        amount: Some(20.1234),
    };

    process_transaction(tx, &mut clients, &mut processed_txs, &mut held_txs).unwrap();

    let tx = Transaction {
        tx_type: TransactionType::Deposit,
        client: 1,
        id: 2,
        amount: Some(1.0007),
    };

    process_transaction(tx, &mut clients, &mut processed_txs, &mut held_txs).unwrap();

    let client = clients.get(&1).unwrap();
    assert_eq!(client.total, 21.1241);
}
