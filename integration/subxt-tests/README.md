# Solang `subxt` integration test suite

This directroy contains integration tests against a real node using `subxt`.

## How to execute the tests

The test cases requires compiled versions of the contracts from the `polkadot` integration test suite inside the `./outputs` dir. To compile everything, run:

```bash
parallel solang compile -v --target polkadot --wasm-opt z -o ./contracts/ ::: ../polkadot/*.sol ../polkadot/test/*.sol
```

Make sure to start a [solang-substrate-ci node](https://github.com/hyperledger-solang/solang-substrate-ci) or a [substrate-contracts-node](https://github.com/paritytech/substrate-contracts-node) on the test host.

Run only one test at the time against the node by setting the `RUST_TEST_THREADS=1` env var or by passing `-- --test-threads=1 ` to `cargo test`.

```bash
# Execute all test cases
cargo test -- --test-threads=1 
```

## How to upgrade the node metadata
A version upgrade of the node likely requires new metadata. The metadata from a local node can be acquired using [subxt](https://crates.io/crates/subxt) like so:

```bash
subxt metadata --url ws://127.0.0.1:9944 -f bytes > metadata.scale
```
