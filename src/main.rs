use csv::ReaderBuilder;
use serde::Deserialize;

use std::collections::VecDeque;
use std::fs::File;
use std::io::BufReader;

#[derive(Debug, Deserialize)]
enum TransactionType {
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

#[derive(Debug, Deserialize)]
struct Transaction {
    #[serde(rename = "type")]
    tx_type: TransactionType,
    client: u16,
    tx: u32,
    amount: Option<f64>,
}

#[derive(Debug)]
struct Client {
    client: u16,
    available: f64,
    held: f64,
    total: f64,
    locked: bool,
}

fn main() {
    //Parse args
    //(were this program to need more complex argument parsing,
    //I would consider the clap crate, but it seemed overkill for this case)
    let args: Vec<String> = std::env::args().collect();
    //Validate number of args
    if args.len() > 2 {
        eprintln!("Usage: {} <input.csv>", args[0]);
        std::process::exit(1);
    }
    let filename = &args[1];

    let input_file = File::open(filename).expect("file to exist");
    let input_buf = BufReader::new(input_file);

    //configure reader to trim whitespace
    let mut rdr = ReaderBuilder::new()
        .trim(csv::Trim::All)
        .delimiter(b',')
        .from_reader(input_buf);

    for result in rdr.deserialize() {
        let record: Transaction = result.expect("a valid record");
        println!("{:?}", record);
    }
}

struct RingBuffer<T> {
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

    pub fn pop(&mut self) -> Option<T> {
        self.inside.pop_front()
    }
}

impl RingBuffer<Transaction> {
    pub fn get_by_tx(&self, id: u32) -> Option<&Transaction> {
        self.inside.iter().find(|x| x.tx == id)
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
