Solang Solidity Examples
========================

Here are two examples of Solidity contracts.


General examples
----------------

Flipper
_______

This is the `ink! flipper example <https://github.com/paritytech/ink/blob/v3.3.0/examples/flipper/lib.rs>`_
written in Solidity:

.. include:: ../examples/polkadot/flipper.sol
  :code: solidity

Example
_______

A few simple arithmetic functions.

.. include:: ../examples/example.sol
  :code: solidity


Solana examples
---------------

NFT example
___________

There is an example on Solana's integration tests for a Solidity contract that manages an NFT. The contract is supposed
to be the NFT itself. It can mint itself and transfer ownership. It also stores on chain information about itself, such as its URI.
Please, check `simple_collectible.sol <https://github.com/hyperledger-solang/solang/blob/main/integration/solana/simple_collectible.sol>`_
for the Solidity contract and `simple_collectible.spec.ts <https://github.com/hyperledger-solang/solang/blob/main/integration/solana/simple_collectible.spec.ts>`_
for the Typescript code that interacts with Solidity.


PDA Hash Table
______________

On Solana, it is possible to create a hash table on chain with program derived addresses (PDA). This is done by
using the intended key as the seed for finding the PDA. There is an example of how one can achieve so in our integration
tests. Please, check `UserStats.sol <https://github.com/hyperledger-solang/solang/blob/main/integration/solana/UserStats.sol>`_
for the Solidity contract and `user_stats.spec.ts <https://github.com/hyperledger-solang/solang/blob/main/integration/solana/user_stats.spec.ts>`_
for the client code, which contains most of the explanations about how the table works. This example was inspired by
`Anchor's PDA hash table <https://www.anchor-lang.com/docs/pdas#hashmap-like-structures-using-pd-as>`_.