use crate::datatypes::{Client, RingBuffer, Transaction, TransactionType};
use csv::{ReaderBuilder, Writer};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;

mod datatypes;
#[cfg(test)]
mod tests;

///Processes a CSV of transactions and outputs the final state of all clients
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

    //Use a buffered reader to read the input file to avoid
    //making a system call for each iteration of the main loop
    let input_buf = BufReader::new(input_file);

    //configure csv reader
    let mut csv_reader = ReaderBuilder::new()
        .trim(csv::Trim::All)
        .delimiter(b',')
        .from_reader(input_buf);

    //Create relevant mutable state to store and update client records, processed transactions,
    //and held transactions
    let mut clients: HashMap<u16, Client> = HashMap::new();
    let mut processed_txs: RingBuffer<Transaction> = RingBuffer::with_capacity(10000);
    let mut held_txs: HashMap<u32, Transaction> = HashMap::new();

    //Process each transaction in the input and update the state of the clients

    //For each transaction record, if it deserializes correctly, process the transaction.
    //Or if errors are returned, ignore the transaction and continue to the next one
    for csv_result in csv_reader.deserialize::<Transaction>() {
        //map_err is used to convert the csv::Error to a String
        //to avoid unnecessary error handling complexity
        let process_result = csv_result.map_err(|e| e.to_string()).and_then(|tx_record| {
            process_transaction(tx_record, &mut clients, &mut processed_txs, &mut held_txs)
        });

        if let Err(_e) = process_result {
            //If debugging, uncomment to print errors to stderr:
            //eprintln!("{_e}");
        }
    }

    //Round the clients' fund values to 4 decimal places
    for client in clients.values_mut() {
        client.available = (client.available * 10000.0f64).round() / 10000.0f64;
        client.total = (client.total * 10000.0f64).round() / 10000.0f64;
        client.held = (client.held * 10000.0f64).round() / 10000.0f64;
    }

    //Create the csv writer
    let mut csv_writer = Writer::from_writer(std::io::stdout());

    //Serialize the client records to stdout
    //Since row order is irrelevant, iterating over
    //the hashmap values is sufficient
    for client in clients.values() {
        csv_writer
            .serialize(client)
            //Expect is used here as the serialization should not fail
            .expect("CSV serialization to succeed");
    }
}

/// Processes a transaction record and updates the client, processed transactions,
/// and held transactions state accordingly
///
/// Errors are returned as strings to be printed to stderr
fn process_transaction(
    tx: Transaction,
    clients: &mut HashMap<u16, Client>,
    processed_txs: &mut RingBuffer<Transaction>,
    held_txs: &mut HashMap<u32, Transaction>,
) -> Result<(), String> {
    match tx.tx_type {
        TransactionType::Deposit => {
            //Get the client record from the hashmap, or create a new one
            let client = clients
                .entry(tx.client)
                .or_insert_with(|| Client::new(tx.client));

            //Unwrap the amount or return an error if it doesn't exist
            let amount = tx
                .amount
                .ok_or_else(|| format!("Deposit transaction missing amount: {tx:?}"))?;

            //increment the client's available and total funds
            client.available += amount;
            client.total += amount;

            //push the processed transaction into the buffer for future
            //reference if needed
            processed_txs.push(tx);
        }
        TransactionType::Withdrawal => {
            //Get the client record from the hashmap, or create a new one
            let client = clients
                .entry(tx.client)
                .or_insert_with(|| Client::new(tx.client));

            //Unwrap the amount or return an error if it doesn't exist
            let amount = tx
                .amount
                .ok_or_else(|| format!("Withdrawal transaction missing amount: {tx:?}"))?;

            //Check if the client has enough funds to withdraw.
            //This will also catch a new client trying to withdraw
            //before depositing, but perhaps that should be a separate error ?
            if client.available < amount {
                return Err(format!("Insufficient funds for withdrawal: {tx:?}"));
            }

            //Decrement the client's available and total funds
            client.available -= amount;
            client.total -= amount;

            //Push the processed transaction into the buffer for future
            //reference if needed
            processed_txs.push(tx);
        }
        TransactionType::Dispute => {
            //Lookup the transaction referenced by the dispute
            let disputed_tx = processed_txs
                .get_by_tx(tx.id)
                .ok_or_else(|| format!("Dispute references non-existent transaction: {tx:?}"))?;

            //Get the client record from the hashmap. This should always exist
            //but check error just for safety
            let client = clients
                .get_mut(&disputed_tx.client)
                .ok_or_else(|| format!("Dispute references non-existent client: {tx:?}"))?;

            //Check that the disputed transaction is a deposit or withdrawal
            if disputed_tx.tx_type != TransactionType::Deposit
                && disputed_tx.tx_type != TransactionType::Withdrawal
            {
                return Err(format!(
                    "Dispute references non-deposit/withdrawal transaction: {tx:?}"
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
                .ok_or_else(|| format!("Resolve references non-existent dispute: {tx:?}"))?;

            //Get the client record from the hashmap. This should always exist
            //but check error just for safety
            let client = clients
                .get_mut(&disputed_tx.client)
                .ok_or_else(|| format!("Resolve references non-existent client: {tx:?}"))?;

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
                .ok_or_else(|| format!("Chargeback references non-existent dispute: {tx:?}"))?;

            //Get the client record from the hashmap. This should always exist
            //but check error just for safety
            let client = clients
                .get_mut(&disputed_tx.client)
                .ok_or_else(|| format!("Chargeback references non-existent client: {tx:?}"))?;

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
