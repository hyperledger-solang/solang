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

A file called `flipper.abi` and `flipper.so`. Now install `@solana/solidity`:

```
npm install @solana/solidity
```

Save the following to `flipper.js`:
```javascript
const { Connection, LAMPORTS_PER_SOL, Keypair } = require('@solana/web3.js');
const { Contract, Program } = require('@solana/solidity');
const { readFileSync } = require('fs');

const FLIPPER_ABI = JSON.parse(readFileSync('./flipper.abi', 'utf8'));
const PROGRAM_SO = readFileSync('./flipper.so');

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

Run:

```bash
solang compile --target substrate flipper.sol
```

Alternatively if you want to use the solang container, run:

```
docker run --rm -it -v $(pwd):/sources ghcr.io/hyperledger/solang compile -v -o /sources --target substrate /sources/flipper.sol
```
You will have a file called flipper.contract. You can use this directly in
the [Contracts UI](https://contracts-ui.substrate.io/),
as if your smart contract was written using ink!.

## Tentative roadmap

Solang has a high level of compatibility with many blockchains. We are trying to ensure the compiler stays
up to date with the newest Solidity syntax and features.  In addition, we focus on bringing new performance optimizations
and improve developer experience.
Here is a brief description of what we envision for the next versions.

### V0.3

| Milestone                                  | Status      |
|--------------------------------------------|-------------|
| Call Solana's Rust contracts from Solidity | Completed   |
| Improvements in overflow checking          | Completed   |
| Support Solana's Program Derived Addresses | Completed   |
| Call Solidity from Solana's Rust contracts | Not started |
| Improve developer experience for Substrate | Not started |
| Tooling for calls between ink! <> solidity | Not started |
| Support chain extensions for Substrate     | Not started |
| Provide CLI for node interactions          | Not started |


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
