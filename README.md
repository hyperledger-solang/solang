<img src="https://raw.githubusercontent.com/hyperledger/solang/main/docs/hl_solang_horizontal-color.svg" alt="Solang Logo" width="75%"/>

# solang - Solidity Compiler for Solana and Polkadot

[![Discord](https://img.shields.io/discord/905194001349627914?logo=Hyperledger&style=plastic)](https://discord.gg/hyperledger)
[![CI](https://github.com/hyperledger-solang/solang/workflows/test/badge.svg)](https://github.com/hyperledger-solang/solang/actions)
[![Documentation Status](https://readthedocs.org/projects/solang/badge/?version=latest)](https://solang.readthedocs.io/en/latest/?badge=latest)
[![license](https://img.shields.io/github/license/hyperledger/solang.svg)](LICENSE)
[![LoC](https://tokei.rs/b1/github/hyperledger/solang?category=lines)](https://github.com/hyperledger-solang/solang)

Welcome to Solang, a new Solidity compiler written in rust which uses
llvm as the compiler backend. Solang can compile Solidity for Solana and
Polkadot Parachains with the `contracts` pallet.
Solang is source compatible with Solidity 0.8,
with some caveats due to differences in the underlying blockchain.

Solang is under active development right now, and has
[extensive documentation](https://solang.readthedocs.io/en/latest/).

## Solana

Please follow the [Solang Getting Started Guide](https://solana.com/developers/guides/solang/getting-started).

Solang is part of the [Solana Tools Suite](https://docs.solana.com/cli/install-solana-cli-tools) (version v1.16.3 and higher).
There is no need to install it separately.

## Installation

Solang is available as a Brew cask for MacOS, with the following command:

```
brew install hyperledger/solang/solang
```

For other operating systems, please check the [installation guide](https://solang.readthedocs.io/en/latest/installing.html).

## Build for Polkadot

Run the following command, selecting the flipper example available on Solang's repository:

```bash
solang compile --target polkadot examples/polkadot/flipper.sol
```

Alternatively if you want to use the solang container, run:

```
docker run --rm -it -v $(pwd):/sources ghcr.io/hyperledger/solang compile -v -o /sources --target polkadot /sources/flipper.sol
```
You will have a file called flipper.contract. You can use this directly in
the [Contracts UI](https://contracts-ui.substrate.io/),
as if your smart contract was written using ink!.

## Tentative roadmap

Solang has a high level of compatibility with many blockchains. We are trying to ensure the compiler stays
up to date with the newest Solidity syntax and features.  In addition, we focus on bringing new performance optimizations
and improve developer experience.
Here is a brief description of what we envision for the next versions.

### V0.4

| Feature                                            | Status                                               |
|----------------------------------------------------|------------------------------------------------------|
| Improve management over optimization passes        | Not started                                          |
| Adopt single static assignment for code generation | In progress                                          |
| Support openzeppelin on Polkadot target            | In progress                                          |
| Provide Solidity -> Polkadot porting guide         | Not started                                          |
| Declare accounts for a Solidity function on Solana | In progress                                          |
| Tooling for calls between ink! <> solidity         | In progress                                          |
| Provide CLI for node interactions                  | [Done](https://github.com/hyperledger-solang/solang-aqd)    |

## License

[Apache 2.0](LICENSE)
