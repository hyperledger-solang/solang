Solang Solidity Compiler
========================

Welcome to the Solang Solidity Compiler. Using Solang, you can compile smart contracts written in
`Solidity <https://en.wikipedia.org/wiki/Solidity>`_ for
`Solana <https://www.solana.com/>`_,
`Parity Substrate <https://substrate.dev/>`_, and
`Ethereum ewasm <https://github.com/ewasm/design>`_. It uses the
`llvm <https://www.llvm.org/>`_ compiler framework to produce WebAssembly
(wasm) or BPF contract code. As result, the output is highly optimized, which saves you in gas costs.

Solang aims for source file compatibility with the Ethereum EVM Solidity compiler,
version 0.7. Where differences exists, this is noted in the language documentation.
The source code repository can be found on `github <https://github.com/hyperledger-labs/solang>`_
and we have a `channel #solang on Hyperledger Discord <https://discord.gg/jhn4rkqNsT>`_, and
a `channel #solang-solidity-compiler on Solana Discord <https://discord.gg/TmE2Ek5ZbW>`_.

Contents
========

.. toctree::
   :maxdepth: 3
   :caption: Using Solang

   installing
   running
   extension
   examples

.. toctree::
   :maxdepth: 3
   :caption: Targets

   targets/solana.rst
   targets/substrate.rst
   targets/burrow.rst

.. toctree::
   :maxdepth: 3
   :caption: Solidity language

   language/introduction.rst
   language/file_structure.rst
   language/imports.rst
   language/pragmas.rst
   language/types.rst
   language/expressions.rst
   language/statements.rst
   language/constants.rst
   language/using.rst
   language/contracts.rst
   language/contract_storage.rst
   language/interface_libraries.rst
   language/events.rst
   language/functions.rst
   language/managing_values.rst
   language/builtins.rst
   language/tags.rst

.. toctree::
   :maxdepth: 3
   :caption: Extras

   optimizer
   testing
   contributing

