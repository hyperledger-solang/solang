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
version 0.7. Where differences exists, this is noted in the :ref:`language documentation <language>`.
The source code repository can be found on `github <https://github.com/hyperledger-labs/solang>`_
and we have a `channel #solang on Hyperledger Discord <https://discord.gg/hyperledger>`_, and
a `channel #solang-solidity-compiler on Solana Discord <https://discord.gg/Solana>`_.

.. toctree::
   :maxdepth: 3
   :caption: Contents:

   installing
   running
   language
   targets
   optimizer
   extension
   examples
   testing
   contributing
