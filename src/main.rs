use crate::datatypes::{Client, RingBuffer, Transaction, TransactionType};
use csv::{ReaderBuilder, Writer};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;

mod datatypes;

fn main() {
    //Parse args
    let args: Vec<String> = std::env::args().collect();

    //Validate number of args
    if args.len() > 2 {
        eprintln!("Usage: {} <input.csv>", args[0]);
        std::process::exit(1);
    }

    //Open the input file, if it doesn't exist, panic
    let input_file = File::open(&args[1]).expect("file to exist");
    let input_buf = BufReader::new(input_file);

    //configure csv reader
    let mut csv_reader = ReaderBuilder::new()
        .trim(csv::Trim::All)
        .delimiter(b',')
        .from_reader(input_buf);

    //Create relevant mutable state to store and update client records, processed transactions,
    //and held transactions
    let mut clients: HashMap<u16, Client> = HashMap::new();
    let mut processed_txs: RingBuffer<Transaction> = RingBuffer::with_capacity(1000);
    let mut held_txs: HashMap<u32, Transaction> = HashMap::new();

    //Process each transaction in the input and update the state of the clients

    //For each transaction record, if it deserializes correctly, process the transaction.
    //Or if errors are returned, error to stderr and continue to the next record
    for result in csv_reader.deserialize::<Transaction>() {
        //map_err is used to convert the csv::Error to a String
        //to avoid increased error handling complexity
        let result = result.map_err(|e| e.to_string()).and_then(|tx_record| {
            process_transaction(tx_record, &mut clients, &mut processed_txs, &mut held_txs)
        });

        if let Err(e) = result {
            eprintln!("{}", e);
        }
    }

    //Create the csv writer
    let mut csv_writer = Writer::from_writer(std::io::stdout());

    //Serialize the client records stdout
    for client in clients.values() {
        csv_writer
            .serialize(client)
            //Expect is used here as the serialization should not fail
            .expect("CSV serialization failed");
    }
}

fn process_transaction(
    tx: Transaction,
    clients: &mut HashMap<u16, Client>,
    processed_txs: &mut RingBuffer<Transaction>,
    held_txs: &mut HashMap<u32, Transaction>,
) -> Result<(), String> {
    match tx.tx_type {
        TransactionType::Deposit => {
            // Get the client record from the hashmap, or create a new one
            let client = clients.entry(tx.client).or_insert(Client::new(tx.client));

            // Check if the transaction has a valid amount
            if tx.amount.is_none() {
                return Err(format!("Deposit transaction missing amount: {:?}", tx));
            }

            //Unwrap the amount, as we've ensured it isn't None, and
            //increment the client's available and total funds
            let amount = tx.amount.unwrap();
            client.available += amount;
            client.total += amount;

            //push the processed transaction into the buffer for future
            //reference if needed
            processed_txs.push(tx);
        }
        TransactionType::Withdrawal => {
            //Get the client record from the hashmap, or create a new one
            let client = clients.entry(tx.client).or_insert(Client::new(tx.client));

            //Check if the transaction has a valid amount
            if tx.amount.is_none() {
                return Err(format!("Withdrawal transaction missing amount: {:?}", tx));
            }

            //Unwrap the amount, as we've ensured it isn't None
            let amount = tx.amount.unwrap();

            //Check if the client has enough funds to withdraw.
            //This will also catch a new client trying to withdraw
            //before depositing, but perhaps that should be a separate error
            if client.available < amount {
                return Err(format!("Insufficient funds for withdrawal: {:?}", tx));
            }

            //Decrement the client's available and total funds
            client.available -= amount;
            client.total -= amount;

            //push the processed transaction into the buffer for future
            //reference if needed
            processed_txs.push(tx);
        }
        TransactionType::Dispute => {
            //Lookup the transaction referenced by the dispute
            let disputed_tx = processed_txs
                .get_by_tx(tx.id)
                .ok_or_else(|| format!("Dispute references non-existent transaction: {:?}", tx))?;

            //Get the client record from the hashmap. This should always exist
            //but check error just for safety
            let client = clients
                .get_mut(&disputed_tx.client)
                .ok_or_else(|| format!("Dispute references non-existent client: {:?}", tx))?;

            //Check that the disputed transaction is a deposit or withdrawal
            if disputed_tx.tx_type != TransactionType::Deposit
                && disputed_tx.tx_type != TransactionType::Withdrawal
            {
                return Err(format!(
                    "Dispute references non-deposit/withdrawal transaction: {:?}",
                    tx
                ));
            }

            //Unwrap the amount, as we've ensured it exists if the transaction
            //is a deposit or withdrawal
            let amount = disputed_tx.amount.unwrap();

            //Decrease the available funds by the amount of the disputed transaction
            client.available -= amount;
            //Increase the held funds by the amount of the disputed transaction
            client.held += amount;

            //Store a copy of the disputed transaction in the held_txs hashmap
            //for easier future reference
            held_txs.insert(tx.id, disputed_tx.clone());
        }
        TransactionType::Resolve => {
            //Lookup the transaction referenced by the resolve
            let disputed_tx = held_txs
                .remove(&tx.id)
                .ok_or_else(|| format!("Resolve references non-existent dispute: {:?}", tx))?;

            //Get the client record from the hashmap. This should always exist
            //but check error just for safety
            let client = clients
                .get_mut(&disputed_tx.client)
                .ok_or_else(|| format!("Resolve references non-existent client: {:?}", tx))?;

            //Unwrap the amount, as we've ensured it exists if the transaction
            //is in the disputed txs hashmap
            let amount = disputed_tx.amount.unwrap();

            //Decrease the held funds by the amount of the disputed transaction
            client.held -= amount;
            //Increase the available funds by the amount of the disputed transaction
            client.available += amount;

            //Remove the disputed transaction from the held_txs hashmap
            held_txs.remove(&disputed_tx.id);
        }
        TransactionType::Chargeback => {
            //Lookup the transaction referenced by the chargeback
            let disputed_tx = held_txs
                .remove(&tx.id)
                .ok_or_else(|| format!("Chargeback references non-existent dispute: {:?}", tx))?;

            //Get the client record from the hashmap. This should always exist
            //but check error just for safety
            let client = clients
                .get_mut(&disputed_tx.client)
                .ok_or_else(|| format!("Chargeback references non-existent client: {:?}", tx))?;

            //Unwrap the amount, as we've ensured it exists if the transaction
            //is in the disputed txs hashmap
            let amount = disputed_tx.amount.unwrap();

            //Decrease the held funds by the amount of the disputed transaction
            client.held -= amount;
            //Decrease the total funds by the amount of the disputed transaction
            client.total -= amount;

            //Set the client's account to locked
            client.locked = true;

            //Remove the disputed transaction from the held_txs hashmap
            held_txs.remove(&disputed_tx.id);
        }
    }
    Ok(())
}
