<img src="https://raw.githubusercontent.com/hyperledger/solang/main/docs/hl_solang_horizontal-color.svg" alt="Solang Logo" width="75%"/>

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

```bash
docker run --rm -it -v $(pwd):/sources ghcr.io/hyperledger/solang compile -v -o /sources --target solana /sources/solana/flipper.sol
```

This generates a file called `flipper.json` and `flipper.so`. In order to deploy the contract code to the account
`F1ipperKF9EfD821ZbbYjS319LXYiBmjhzkkf5a26rC`, save the private key to the file `flipper-keypair.json`:

```bash
echo "[4,10,246,143,43,1,234,17,159,249,41,16,230,9,198,162,107,221,233,124,34,15,16,57,205,53,237,217,149,17,229,195,3,150,242,90,91,222,117,26,196,224,214,105,82,62,237,137,92,67,213,23,14,206,230,155,43,36,85,254,247,11,226,145]" > flipper-keypair.json
 ```

Now you can deploy the contract code using:

```bash
solana program deploy flipper.so
```

Now install `@project-serum/anchor`:

```
npm install @project-serum/anchor
```

Save the following to `flipper.js`:
```javascript
const { readFileSync } = require('fs');
const anchor = require('@project-serum/anchor');

const IDL = JSON.parse(readFileSync('./flipper.json', 'utf8'));
const PROGRAM_SO = readFileSync('./flipper.so');

(async function () {
	const provider = anchor.AnchorProvider.env();

	const dataAccount = anchor.web3.Keypair.generate();

	const programId = new anchor.web3.PublicKey(IDL.metadata.address);

	const wallet = provider.wallet.publicKey;

	const program = new anchor.Program(IDL, programId, provider);

	await program.methods.new(wallet, true)
		.accounts({ dataAccount: dataAccount.publicKey })
		.signers([dataAccount]).rpc();

	const val1 = await program.methods.get()
		.accounts({ dataAccount: dataAccount.publicKey })
		.view();

	console.log(`state: ${val1}`);

	await program.methods.flip()
		.accounts({ dataAccount: dataAccount.publicKey })
		.rpc();

	const val2 = await program.methods.get()
		.accounts({ dataAccount: dataAccount.publicKey })
		.view();

	console.log(`state: ${val2}`);
})();
```
And now run:
```
export ANCHOR_WALLET=$HOME/.config/solana/id.json
export ANCHOR_PROVIDER_URL=http://127.0.0.1:8899
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
| Improve developer experience for Substrate | In progress |
| Tooling for calls between ink! <> solidity | In progress |
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
