# solang - A Solidity to wasm compiler written in rust

[![CI](https://github.com/hyperledger-labs/solang/workflows/test/badge.svg)](https://github.com/hyperledger-labs/solang/actions)
[![Documentation Status](https://readthedocs.org/projects/solang/badge/?version=latest)](https://solang.readthedocs.io/en/latest/?badge=latest)
[![LoC](https://tokei.rs/b1/github/hyperledger-labs/solang?category=lines)](https://github.com/hyperledger-labs/solang)

[<img align="right" width="640" src="docs/web3_foundation_grants_badge_black.svg" alt="Funded by the web3 foundation">](https://github.com/w3f/Web3-collaboration/blob/master/grants/accepted_grant_applications.md#wave-4)

Welcome to Solang, a new Solidity compiler written in rust which uses
llvm as the compiler backend. As a result, only the compiler front end
needs to be written in rust.

Solang targets Substrate, ewasm, and Sawtooth.

Solang is under active development right now, and should be documented at
the same time as the implementation. Please have a look at
[our documentation](https://solang.readthedocs.io/en/latest/).

## Solang Hyperledger Mentorship

Solang has been accepted in the
[Hyperledger Mentorship Program](https://wiki.hyperledger.org/display/INTERN/Create+a+new+Solidity+Language+Server+%28SLS%29+using+Solang+Compiler).
The Mentorship Program exists to encourage students to contribute to Hyperledger
open source projects. Hyperledger projects provide mentors and the Hyperledger
organization gives some money to participating students.

If you would like mentorship, please apply before the 24th of April 2020.

Looking forward to your applications!

## Simple example

First build [Solang](https://solang.readthedocs.io/en/latest/installing.html)
or use the docker image, then write the following to flipper.sol:

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

Now run:

```bash
solang flipper.sol
```

Alternatively if you want to use the solang docker image, run:

```
docker run --rm -it -v $(pwd):/sources hyperledgerlabs/solang -v -o /sources /sources/flipper.sol
```
You will have a flipper.wasm and flipper.json. You can use these directly in
the [Polkadot UI](https://substrate.dev/substrate-contracts-workshop/#/0/deploying-your-contract?id=putting-your-code-on-the-blockchain), as if your smart
contract was written using ink!.
