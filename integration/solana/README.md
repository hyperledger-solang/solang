<p align="center">
  <a href="https://solana.com">
    <img alt="Solana" src="https://i.imgur.com/OMnvVEz.png" width="250" />
  </a>
</p>

[![Build status][travis-image]][travis-url]
[![Gitpod Ready-to-Code](https://img.shields.io/badge/Gitpod-Ready--to--Code-blue?logo=gitpod)](https://gitpod.io/#https://github.com/solana-labs/example-helloworld)

[travis-image]: https://travis-ci.org/solana-labs/example-helloworld.svg?branch=master
[travis-url]: https://travis-ci.org/solana-labs/example-helloworld

# Hello world on Solana

This project demonstrates how to use the [Solana Javascript API](https://github.com/solana-labs/solana-web3.js)
to build, deploy, and interact with programs on the Solana blockchain.

The project comprises of:

* An on-chain hello world program
* A client that can send a "hello" to an account and get back the number of times "hello" has been sent

## Table of Contents
- [Hello world on Solana](#hello-world-on-solana)
  - [Table of Contents](#table-of-contents)
  - [Quick Start](#quick-start)
    - [Start local Solana cluster](#start-local-solana-cluster)
    - [Build the on-chain program](#build-the-on-chain-program)
    - [Run the client](#run-the-client)
    - [Expected output](#expected-output)
      - [Not seeing the expected output?](#not-seeing-the-expected-output)
    - [Customizing the Program](#customizing-the-program)
  - [Learn about Solana](#learn-about-solana)
  - [Learn about the client](#learn-about-the-client)
    - [Entrypoint](#entrypoint)
    - [Establish a connection to the cluster](#establish-a-connection-to-the-cluster)
    - [Load the helloworld on-chain program if not already loaded](#load-the-helloworld-on-chain-program-if-not-already-loaded)
    - [Send a "Hello" transaction to the on-chain program](#send-a-%22hello%22-transaction-to-the-on-chain-program)
    - [Query the Solana account used in the "Hello" transaction](#query-the-solana-account-used-in-the-%22hello%22-transaction)
  - [Learn about the on-chain program](#learn-about-the-on-chain-program)
    - [Entrypoint](#entrypoint-1)
    - [Processing an instruction](#processing-an-instruction)
    - [Rust limitations](#rust-limitations)
  - [Pointing to a public Solana cluster](#pointing-to-a-public-solana-cluster)
  - [Expand your skills with advanced examples](#expand-your-skills-with-advanced-examples)

## Quick Start

[![Open in Gitpod](https://gitpod.io/button/open-in-gitpod.svg)](https://gitpod.io/#https://github.com/solana-labs/example-helloworld)

If you decide to open in Gitpod then refer to [README-gitpod.md](README-gitpod.md), otherwise continue reading.

The following dependencies are required to build and run this example,
depending on your OS, they may already be installed:

```bash
$ node --version
$ npm --version
$ docker -v
$ wget --version
$ rustup --version
$ rustc --version
$ cargo --version
```

If this is your first time using Docker or Rust, these [Installation Notes](README-installation-notes.md) might be helpful.

### Start local Solana cluster

This example connects to a local Solana cluster by default.

Enable on-chain program logs:
```bash
$ export RUST_LOG=solana_runtime::system_instruction_processor=trace,solana_runtime::message_processor=info,solana_bpf_loader=debug,solana_rbpf=debug
```

Start a local Solana cluster:
```bash
$ npm run localnet:update
$ npm run localnet:up
```

View the cluster logs:
```bash
$ npm run localnet:logs
```

Note: To stop the local Solana cluster later:
```bash
$ npm run localnet:down
```

### Build the on-chain program

There is both a Rust and C version of the on-chain program, whichever is built last will be the one used when running the example.

```bash
$ npm run build:program-rust
```

```bash
$ npm run build:program-c
```

### Run the client

```bash
$ npm run start
```

### Expected output

Public key values will differ:

```bash
Lets say hello to a Solana account...
Connection to cluster established: http://localhost:8899 { solana-core: 1.1.2 }
Loading hello world program...
Program loaded to account 47bZX1D1tdmw3KWTo5MfBrAwwHBJQQzQL4VnNGT7HtyQ
Creating account Eys1jdLHdZ2AE56QAKpfadbjziMZ6NAvpL7qsdtM6sbk to say hello to
Saying hello to Eys1jdLHdZ2AE56QAKpfadbjziMZ6NAvpL7qsdtM6sbk
Eys1jdLHdZ2AE56QAKpfadbjziMZ6NAvpL7qsdtM6sbk has been greeted 1 times
Success
```

#### Not seeing the expected output?

- Ensure you've [started the local cluster](#start-local-solana-cluster) and [built the on-chain program](#build-the-on-chain-program).
- Ensure Docker is running.  You might try bumping up its resource settings, 8 GB of memory and 3 GB of swap should help.
- Inspect the Solana cluster logs looking for any failed transactions or failed on-chain programs
  - Expand the log filter and restart the cluster to see more detail
    - ```bash
      $ npm run localnet:down
      $ export RUST_LOG=solana_runtime::native_loader=trace,solana_runtime::system_instruction_processor=trace,solana_runtime::bank=debug,solana_bpf_loader=debug,solana_rbpf=debug
      $ npm run localnet:up

### Customizing the Program

To customize the example, make changes to the files under `/src`.  If you change any files under `/src/program-rust` or `/src/program-c` you will need to [rebuild the on-chain program](#build-the-on-chain-program)

Now when you rerun `npm run start`, you should see the results of your changes.

## Learn about Solana

More information about how Solana works is available in the [Solana documentation](https://docs.solana.com/) and all the source code is available on [github](https://github.com/solana-labs/solana)

Futher questions?  Visit us on [Discord](https://discordapp.com/invite/pquxPsq)

## Learn about the client

The client in this example is written in JavaScript using:
- [Solana web3.js SDK](https://github.com/solana-labs/solana-web3.js)
- [Solana web3 API](https://solana-labs.github.io/solana-web3.js)

### Entrypoint

The [client's entrypoint](https://github.com/solana-labs/example-helloworld/blob/e936ab42e168f1939df0164d5996adf9ca635bd0/src/client/main.js#L14) does four things

### Establish a connection to the cluster

The client establishes a connection with the client by calling [`establishConnection`](https://github.com/solana-labs/example-helloworld/blob/e936ab42e168f1939df0164d5996adf9ca635bd0/src/client/hello_world.js#L45).

### Load the helloworld on-chain program if not already loaded

The process of loading a program on the cluster includes storing the shared object's bytes in a Solana account's data vector and marking the account executable.

The client loads the program by calling [`loadProgram`](https://github.com/solana-labs/example-helloworld/blob/e936ab42e168f1939df0164d5996adf9ca635bd0/src/client/hello_world.js#L54).  The first time `loadProgram` is called the client:

- Read the shared object from the file system
- Calculates the fees associated with loading the program
- Airdrops lamports to a payer account to pay for the load
- Loads the program via the Solana web3.js function ['BPFLoader.load']([TODO](https://github.com/solana-labs/solana-web3.js/blob/37d57926b9dba05d1ad505d4fd39d061030e2e87/src/bpf-loader.js#L36))
- Creates a new "greeter" account that will be used in the "Hello" transaction
- Records the [public key](https://github.com/solana-labs/solana-web3.js/blob/37d57926b9dba05d1ad505d4fd39d061030e2e87/src/publickey.js#L10) of both the loaded helloworld program and the "greeter" account in a config file.  Repeated calls to the client will refer to the same loaded program and "greeter" account.  (To force the reload of the program issue `npm clean:store`)

### Send a "Hello" transaction to the on-chain program

The client then constructs and sends a "Hello" transaction to the program by calling [`sayHello`](https://github.com/solana-labs/example-helloworld/blob/e936ab42e168f1939df0164d5996adf9ca635bd0/src/client/hello_world.js#L121).  The transaction contains a single very simple instruction that primarily caries the public key of the helloworld program account to call and the "greeter" account to which the client wishes to say "Hello" to.

### Query the Solana account used in the "Hello" transaction

Each time the client says "Hello" to an account, the program increments a numerical count in the "greeter" account's data.  The client queries the "greeter" account's data to discover the current number of times the account has been greeted by calling [`reportHellos`](https://github.com/solana-labs/example-helloworld/blob/e936ab42e168f1939df0164d5996adf9ca635bd0/src/client/hello_world.js#L138.)

## Learn about the on-chain program

The [on-chain helloworld program](src/program/Cargo.toml) is a Rust program compiled to [Berkley Packet Format (BPF)](https://en.wikipedia.org/wiki/Berkeley_Packet_Filter) and stored as an [Executable and Linkable Format (ELF) shared object](https://en.wikipedia.org/wiki/Executable_and_Linkable_Format).

The program is written using:
- [Solana Rust SDK](https://github.com/solana-labs/solana/tree/master/sdk)

### Entrypoint

The program's [entrypoint](https://github.com/solana-labs/example-helloworld/blob/6508bdb54c4d7f60747263b4274283fbddfabffe/src/program/src/lib.rs#L12) takes three parameters:

```rust
fn process_instruction<'a>(
    program_id: &Pubkey, // Public key of the account the hello world program was loaded into
    accounts: &'a [AccountInfo<'a>], // The account to say hello to
    _instruction_data: &[u8], // Ignored, all helloworld instructions are hellos
) -> ProgramResult {
```

- `program_id` is the public key of the currently executing program.  The same program can be uploaded to the cluster under different accounts, and a program can use `program_id` to determine which instance of the program is currently executing.
- `accounts` is a slice of [`Account Info's](https://github.com/solana-labs/solana/blob/b4e00275b2da6028cc839a79cdc4453d4c9aca13/sdk/src/account_info.rs#L10) representing each account included in the instruction being processed.
- `_instruction_data` is a data vector containing the [data passed as part of the instruction](https://github.com/solana-labs/solana-web3.js/blob/37d57926b9dba05d1ad505d4fd39d061030e2e87/src/transaction.js#L46).  In the case of helloworld no instruction data is passed and thus ignored (all instructions are treated as a "Hello" instruction).  Typically the instruction data would contain information about what kind of command the program should process and details about that particular command.

### Processing an instruction

Given the inputs to the entrypoint, the result of the instruction are updates to account's lamports and data vectors.  In the case of helloworld, the "greeted" account's data holds a 32-bit Little-endian encoded unsigned integer, which gets incremented.

The program does a series of checks to ensure that the instruction is well-formed (the "greeted" account is owned by the program and has sufficient data to hold a 32-bit unsigned integer).

The accounts slice may contain the same account in multiple positions, so a Rust ` std protects any writable data::cell::RefCell`

The program prints a diagnostic message to the validators' logs by calling [`info!`](https://github.com/solana-labs/solana/blob/b4e00275b2da6028cc839a79cdc4453d4c9aca13/sdk/src/log.rs#L12).  On a local cluster you can view the logs by including `solana_bpf_loader_program=info` in `RUST_LOG`.

If the program fails, it returns a `ProgramError`; otherwise, it returns `Ok(())` to indicate to the runtime that any updates to the accounts may be recorded on the chain.

### Rust limitations

On-chain Rust programs support most of Rust's libstd, libcore, and liballoc, as well as many 3rd party crates.

There are some limitations since these programs run in a resource-constrained, single-threaded environment, and must be deterministic:

- No access to
  - `rand` or any crates that depend on it
  - `std::fs`
  - `std::net`
  - `std::os`
  - `std::future`
  - `std::net`
  - `std::process`
  - `std::sync`
  - `std::task`
  - `std::thread`
  - `std::time`
- Limited access to:
  - `std::hash`
  - `std::os`
- Bincode is extreamly computationally expensive in both cycles and call depth and should be avoided
- String formating should be avoided since it is also computationaly expensive
- No support for `println!`, `print!`, the Solana SDK helpers in `src/log.rs` should be used instead
- The runtime enforces a limit on the number of instructions a program can execute during the processing of one instruction

## Pointing to a public Solana cluster

Solana maintains three public clusters:
- `devnet` - Development cluster with airdrops enabled
- `testnet` - Tour De Sol test cluster without airdrops enabled
- `mainnet-beta` -  Main cluster
  
Use npm scripts to configure which cluster.

To point to `devnet`:
```bash
$ npm run cluster:devnet
```

To point back to the local cluster:
```bash
$ npm run cluster:localnet
```

## Expand your skills with advanced examples

There is lots more to learn; The following examples demonstrate more advanced features like custom errors, advanced account handling, suggestions for data serialization, benchmarking, etc..

- [ERC-20-like Token](https://github.com/solana-labs/example-token)
- [TicTacToe](https://github.com/solana-labs/example-tictactoe)
- [MessageFeed](https://github.com/solana-labs/example-messagefeed)
