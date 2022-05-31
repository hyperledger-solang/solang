# solang - Solidity Compiler for Solana, Substrate, and ewasm

[![Discord](https://img.shields.io/discord/905194001349627914?logo=Hyperledger&style=plastic)](https://discord.gg/jhn4rkqNsT)
[![CI](https://github.com/hyperledger-labs/solang/workflows/test/badge.svg)](https://github.com/hyperledger-labs/solang/actions)
[![Documentation Status](https://readthedocs.org/projects/solang/badge/?version=latest)](https://solang.readthedocs.io/en/latest/?badge=latest)
[![license](https://img.shields.io/github/license/hyperledger-labs/solang.svg)](LICENSE)
[![LoC](https://tokei.rs/b1/github/hyperledger-labs/solang?category=lines)](https://github.com/hyperledger-labs/solang)

Welcome to Solang, a new Solidity compiler written in rust which uses
llvm as the compiler backend. Solang can compile Solidity for Solana,
Substrate, and ewasm. Solang is source compatible with Solidity 0.7, with
some caveats due to differences in the underlying blockchain.

Solang is under active development right now, and has
[extensive documentation](https://solang.readthedocs.io/en/latest/).

## Simple example

First build [Solang](https://solang.readthedocs.io/en/latest/installing.html)
or use the container, then write the following to flipper.sol:

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
solang --target solana flipper.sol
```

Alternatively if you want to use the solang container, run:

```
docker run --rm -it -v $(pwd):/sources ghcr.io/hyperledger-labs/solang -v -o /sources --target solana /sources/flipper.sol
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

    await contract.deploy('flipper', [true], program, storage, 17);

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
solang --target substrate flipper.sol
```

Alternatively if you want to use the solang container, run:

```
docker run --rm -it -v $(pwd):/sources ghcr.io/hyperledger-labs/solang -v -o /sources --target substrate /sources/flipper.sol
```
You will have a file called flipper.contract. You can use this directly in
the [Polkadot UI](https://substrate.dev/substrate-contracts-workshop/#/0/deploy-contract),
as if your smart contract was written using ink!.

## Tentative roadmap

Solang has a high level of compatibility with many blockchains. We are trying to ensure the compiler stays
up to date with the newest Solidity syntax and features.  In addition, we focus on bringing new performance optimizations
and improve developer experience.
Here is a brief description of what we envision for the next versions.

### V0.2

| Milestone | Status      |
| --------- |-------------|
| Specify values as "1 sol" and "1e9 lamports" | In progress |
| Solana SPL tokens compatibility | Completed |

### V0.3

| Milestone | Status      |
| --------- |-------------|
| Call Rust contracts from Solidity | Not started |
| Parse and resolve inline assembly | Completed   |
| Improvements in overflow checking | Not started |


### V0.4

| Milestone                         | Status      |
|-----------------------------------|-------------|
| Call Solidity from Rust           | Not started |
| Generate code for inline assembly | Completed   |
| Improve management over optimization passes | Not Started |
| Dead code elimination | Not started |
| ewasm target | Not started |



## License

[Apache 2.0](LICENSE)
