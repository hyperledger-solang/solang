<img src="docs//hl_solang_horizontal-color.svg" alt="Solang Logo" width="75%"/>

# solang - Solidity Compiler for Solana and Substrate

[![Discord](https://img.shields.io/discord/905194001349627914?logo=Hyperledger&style=plastic)](https://discord.gg/jhn4rkqNsT)
[![CI](https://github.com/hyperledger/solang/workflows/test/badge.svg)](https://github.com/hyperledger/solang/actions)
[![Documentation Status](https://readthedocs.org/projects/solang/badge/?version=latest)](https://solang.readthedocs.io/en/latest/?badge=latest)
[![license](https://img.shields.io/github/license/hyperledger/solang.svg)](LICENSE)
[![LoC](https://tokei.rs/b1/github/hyperledger/solang?category=lines)](https://github.com/hyperledger/solang)

Welcome to Solang, a new Solidity compiler written in rust which uses
llvm as the compiler backend. Solang can compile Solidity for Solana and
Substrate. Solang is source compatible with Solidity 0.8, with
some caveats due to differences in the underlying blockchain.

Solang is under active development right now, and has
[extensive documentation](https://solang.readthedocs.io/en/latest/).


## Installation

Solang is available as a Brew cask for MacOS, with the following command:

```
brew install hyperledger/solang/solang
```

For other operating systems, please check the [installation guide](https://solang.readthedocs.io/en/latest/installing.html).

## Simple example

After installing the compiler, write the following to flipper.sol:

```solidity
contract flipper {
	bool private value;

	constructor(bool initvalue) public {
		value = initvalue;
	}

	function flip() public {
		value = !value;
	}

	function get() public view returns (bool) {
		return value;
	}
}
```

## Build for Solana

Run:

```bash
solang compile --target solana flipper.sol
```

Alternatively if you want to use the solang container, run:

```
docker run --rm -it -v $(pwd):/sources ghcr.io/hyperledger/solang compile -v -o /sources --target solana /sources/flipper.sol
```

A file called `flipper.abi` and `bundle.so`. Now install `@solana/solidity`:

```
npm install @solana/solidity
```

Save the following to `flipper.js`:
```javascript
const { Connection, LAMPORTS_PER_SOL, Keypair } = require('@solana/web3.js');
const { Contract, Program } = require('@solana/solidity');
const { readFileSync } = require('fs');

const FLIPPER_ABI = JSON.parse(readFileSync('./flipper.abi', 'utf8'));
const PROGRAM_SO = readFileSync('./bundle.so');

(async function () {
    console.log('Connecting to your local Solana node ...');
    const connection = new Connection('http://localhost:8899', 'confirmed');

    const payer = Keypair.generate();

    console.log('Airdropping SOL to a new wallet ...');
    const signature = await connection.requestAirdrop(payer.publicKey, LAMPORTS_PER_SOL);
    await connection.confirmTransaction(signature, 'confirmed');

    const program = Keypair.generate();
    const storage = Keypair.generate();

    const contract = new Contract(connection, program.publicKey, storage.publicKey, FLIPPER_ABI, payer);

    await contract.load(program, PROGRAM_SO);

    console.log('Program deployment finished, deploying the flipper contract ...');

    await contract.deploy('flipper', [true], storage, 17);

    const res = await contract.functions.get();
    console.log('state: ' + res.result);

    await contract.functions.flip();

    const res2 = await contract.functions.get();
    console.log('state: ' + res2.result);
})();
```

And now run:
```
node flipper.js
```

## Build for Substrate

### Status
:warning: Solang was developed against Substrate v3.0. It has been a while since the last time the Substrate target was worked on, which introduced a few known regressions. Currently, the following is known to **not** work with recent Substrate versions:

* Function call arguments of type `address`
* Function return values of type `address`
* Cross-contract calls
* Events with indexed fields

Maintenance on the Substrate target has now resumed and we are working on fixing these issues.

### Building
Run:

```bash
solang compile --target substrate flipper.sol
```

Alternatively if you want to use the solang container, run:

```
docker run --rm -it -v $(pwd):/sources ghcr.io/hyperledger/solang -v -o /sources --target substrate /sources/flipper.sol
```
You will have a file called flipper.contract. You can use this directly in
the [Contracts UI](https://contracts-ui.substrate.io/),
as if your smart contract was written using ink!.

## Tentative roadmap

Solang has a high level of compatibility with many blockchains. We are trying to ensure the compiler stays
up to date with the newest Solidity syntax and features.  In addition, we focus on bringing new performance optimizations
and improve developer experience.
Here is a brief description of what we envision for the next versions.

### V0.2

| Milestone                                  | Status      |
|--------------------------------------------|-------------|
| Solana SPL tokens compatibility            | Completed   |
| Parse and resolve inline assembly          | Completed   |
| Generate code for inline assembly          | Completed   |
| Support Solana's Program Derived Addresses | In Progress |
| Support latest Substrate production target | In Progress |


### V0.3

| Milestone                                  | Status      |
|--------------------------------------------|-------------|
| Call Solana's Rust contracts from Solidity | In progress |
| Improvements in overflow checking          | In progress |
| Call Solidity from Solana's Rust contracts | Not started |
| Improve parser resilience                  | Not started |
| Improve developer experience for Substrate | Not started |
| Tooling for calls between ink! <> solidity | Not started |


### V0.4

| Milestone                                          | Status      |
|----------------------------------------------------|-------------|
| Improve management over optimization passes        | Not started |
| Specify values as "1 sol" and "1e9 lamports"       | In progress |
| Adopt single static assignment for code generation | Not started |
| Support openzeppelin on Substrate target           | Not started |
| Provide Solidity -> Substrate porting guide        | Not started |



## License

[Apache 2.0](LICENSE)
