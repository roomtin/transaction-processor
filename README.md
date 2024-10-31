# transaction-processor

This is a simple toy transaction processor that reads in a CSV file and processes the transactions in the file, keeping track of the clients' states involved in the transactions and outputting their final values in CSV format to `stdout` once finished.

## Usage
`cargo run -- <input.csv>`

## Completeness
This implementation handles deposits, withdrawals, disputes, resolves, and chargebacks.

## Correctness
I've worked to model the valid state of the program with structs and enums such that the program shouldn't be able to end up in an invalid state. Since I'm using pattern matching to determine what action to take and because I've derived the CSV serialization from these data structures directly, I'm fairly confident that if a CSV row deserializes correctly, I have logic to handle it.

To verify correctness within each of the types of transactions I wrote unit tests for them, as well as some specific unit tests for other potentially problematic aspects of the program, e.g., verifying custom data structure manipulation and erroring on disallowed actions.

I also created a few different versions of sample input to check against. They are included in the `test_csvs` directory and their expected output is as follows (allowing for irrelevant row ordering):

`basic.csv`:

```
client,available,held,total,locked
2,2.0,0.0,2.0,false
1,1.5,0.0,1.5,false
```

`csv_error_test.csv`:

```
client,available,held,total,locked
1,3.0,0.0,3.0,false
2,1.0,0.0,1.0,false
```

`full_test.csv`:

```
client,available,held,total,locked
12,0.0,0.0,0.0,false
3,1.0,0.0,1.0,false
4,20.0,0.0,20.0,true
1,1.5,0.0,1.5,false
2,-0.0001,2.0,1.9999,false
```

## Safety and Robustness

One of the core assumptions I made was that errors regarding transactions should be logged, and then the program should continue on. It seemed imprudent to halt the continued processing of potentially valid transactions due to encountering an error here and there, so I log most encountered errors to `stderr` and then continue processing input. Errors that intentionally halt the program are those that are unrecoverable, e.g. invalid input file path.

I did not use any `unsafe` features in the implementation. There are a few cases where I made use of `.unwrap()` on `Option` types for convenience, however these cases are covered by there not being a valid path through the program that results in them being unwrapped while containing a `None` state.

### Extension

I represented all my errors as just `String`s, but I find that this begins to cause headache when it's desirable to have conditional logic based on error values or once it becomes necessary to test that a *specific* error is produced from a test case. Were I to extend this program, I would consider representing error states with enums & structs, and/or the use of error assistance crates such as `thiserror` and `anyhow`.

## Efficiency

Due to the potential of a very large input I wanted the program to not require reading the whole input before processing. Furthermore, were it to be bundled in a real time server it wouldn't be possible to do that anyway, but streaming values through memory presents a snag since some transactions reference previously processed ones. I chose to handle this by keeping a fixed size buffer of processed transactions. This has the downside of yielding an error if a dispute, resolve, or chargeback references a transaction so old it's no longer in the buffer. To hopefully avoid this problem, I've initialized the buffer at a generous 10,000 elements, however in a production version of this problem, it might be better to implement a method of storing previously processed transactions to a database so that they can be referenced for as long as they are needed.

I designed my buffer to use a `VecDeque` with a custom `push()` method which removes the oldest element if a new push would exceed the queue's capacity. I considered using a `HashMap` which would have allowed quick lookups for disputed transactions, but ultimately decided the custom `VecDeque` was superior.  A `HashMap` would have required a full search for the oldest element in every push that exceeded the capacity I wanted to maintain. Conversely, the `VecDeque` requires searching to find a (specific) disputed transaction, but since disputes should be a rarer operation than deposits and withdrawals, it didn't make sense to optimize for disputes.

### Extension

Were I to extend this as part of a server and CSV entries came from concurrent TCP streams, the primary change would be to switch to `async` code and `tokio`, making use of tokio's versions of the `std::io` functions.
