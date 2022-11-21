Solang Solidity Examples
========================

Here are two examples of Solidity contracts.


General examples
----------------

Flipper
_______

This is the `ink! flipper example <https://github.com/paritytech/ink/blob/v3.3./examples/flipper/lib.rs>`_
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

There is an example on Solana's integration tests for a Solidity contract that manages an NFT. The contract is supposed
to be the NFT itself. It can mint itself and transfer ownership. It also stores on chain information about itself, such as its URI.
Please, check `simple_collectible.sol <https://github.com/hyperledger/solang/blob/main/integration/solana/simple_collectible.sol>`_
for the Solidity contract and `simple_collectible.spec.ts <https://github.com/hyperledger/solang/blob/main/integration/solana/simple_collectible.spec.ts>`_
for the Typescript code that interacts with Solidity.
