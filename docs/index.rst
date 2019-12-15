Solang Solidity Compiler
========================

.. image:: web3_foundation_grants_badge_black.svg
    :width: 640px
    :alt: Funded by the web3 foundation
    :align: center

Welcome to the Solang Solidity compiler. Using Solang, you can compile
smart contracts written in `Solidity <https://en.wikipedia.org/wiki/Solidity>`_
for `Substrate <https://substrate.dev/>`_ or
`Hyperledger Burrow <https://github.com/hyperledger/burrow>`_. It uses the
`llvm <https://www.llvm.org/>`_ compiler framework to produce WebAssembly
(wasm). As result, the output is highly optimized, which saves you in gas costs.

Solang aims for compatibility with the Ethereum EVM Solidity compiler, but differences
do exist. Where differences exists, this is noted in the documentation.

Many language features are not implemented yet. Anything which is documented
is supported, though.

.. toctree::
   :maxdepth: 3
   :caption: Contents:

   installing
   running
   examples
   language
