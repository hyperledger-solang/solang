.. image:: hl_solang_horizontal-color.svg
    :width: 75%
    :alt: Hyperledger Solang Logo
    :align: center
    :target: https://github.com/hyperledger-solang/solang

|

Solang Solidity Compiler
========================

Welcome to the Solang Solidity Compiler. Using Solang, you can compile smart contracts written in
`Solidity <https://en.wikipedia.org/wiki/Solidity>`_ for
`Solana <https://www.solana.com/>`_ and
`Polkadot <https://substrate.io/>`_. It uses the
`llvm <https://www.llvm.org/>`_ compiler framework to produce WebAssembly
(WASM) or Solana SBF contract code. As result, the output is highly optimized, which saves you in gas costs
or compute units.

Solang aims for source file compatibility with the Ethereum EVM Solidity compiler,
version 0.8. Where differences exist, this is noted in the language documentation.
The source code repository can be found on `github <https://github.com/hyperledger-solang/solang>`_
and we have solang channels on `Hyperledger Discord <https://discord.gg/hyperledger>`_.

Contents
========

.. toctree::
   :maxdepth: 3
   :caption: Using Solang

   installing
   running
   aqd
   extension
   examples

.. toctree::
   :maxdepth: 3
   :caption: Targets

   targets/solana.rst
   targets/polkadot.rst

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
   language/inline_assembly.rst

.. toctree::
   :maxdepth: 3
   :caption: Yul language

   yul_language/yul.rst
   yul_language/statements.rst
   yul_language/types.rst
   yul_language/functions.rst
   yul_language/builtins.rst

.. toctree::
   :maxdepth: 3
   :caption: Extras

   code_gen_options
   testing
   contributing

