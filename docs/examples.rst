Solang Solidity Examples
========================

Here are two examples of Solidity contracts.


General examples
----------------

Flipper
_______

This is the `ink! flipper example <https://github.com/paritytech/ink/blob/v3.3.0/examples/flipper/lib.rs>`_
written in Solidity:

.. include:: ../examples/flipper.sol
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

There is an example on Solana's integration tests for a Solidity contract that manages NFTs.
The contract can mint tokens and transfer ownership. It also saves information related to each NFT, such as its URI and
and an identifier in the example contract. Please, check `simple_collectible.sol <https://github.com/hyperledger/solang/blob/main/integration/solana/simple_collectible.sol>`_
for the Solidity contract and `simple_collectible.spec.ts <https://github.com/hyperledger/solang/blob/main/integration/solana/simple_collectible.spec.ts>`_
for the Typescript code that interacts with Solidity.
