Hyperledger Burrow (ewasm)
==========================

The ewasm specification is not finalized yet. There is no `create2` or `chainid` call, and the keccak256 precompile
contract has not been finalized yet.

In Burrow, Solang is used transparently by the ``burrow deploy`` tool if it is given the ``--wasm`` argument.
When building and deploying a Solidity contract, rather than running the ``solc`` compiler, it will run
the ``solang`` compiler and deploy it as a wasm contract.

This is documented in the `burrow documentation <https://hyperledger.github.io/burrow/#/reference/wasm>`_.

ewasm has been tested with `Hyperledger Burrow <https://github.com/hyperledger/burrow>`_.
Please use the latest master version of burrow, as ewasm support is still maturing in Burrow.

Some language features have not been fully implemented yet on ewasm:

- Contract storage variables types ``string``, ``bytes`` and function types are not implemented
