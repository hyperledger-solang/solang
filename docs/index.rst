Solang Solidity Compiler
========================

.. image:: web3_foundation_grants_badge_black.svg
    :width: 640px
    :alt: Funded by the web3 foundation
    :align: center
    :target: https://github.com/w3f/Web3-collaboration/blob/master/grants/accepted_grant_applications.md#wave-4

Welcome to the Solang Solidity compiler, the portable Solidity compiler.
Using Solang, you can compile smart contracts written in
`Solidity <https://en.wikipedia.org/wiki/Solidity>`_
for `Substrate <https://substrate.dev/>`_,
`Sawtooth Sabre <https://github.com/hyperledger/sawtooth-sabre>`_, and
`Ethereum ewasm <https://github.com/ewasm/design>`_. It uses the
`llvm <https://www.llvm.org/>`_ compiler framework to produce WebAssembly
(wasm). As result, the output is highly optimized, which saves you in gas costs.

Solang aims for source file compatibility with the Ethereum EVM Solidity compiler.
Where differences exists, this is noted in the documentation.

Many language features are not implemented yet. Anything which is documented
is supported. The repository can be found on `github <https://github.com/hyperledger-labs/solang>`_.

Mentorship
----------

Solang has been accepted in the
`Hyperledger Mentorship Program <https://wiki.hyperledger.org/display/INTERN/Create+a+new+Solidity+Language+Server+%28SLS%29+using+Solang+Compiler>`_.
The Mentorship Program exists to encourage students to contribute to Hyperledger
open source projects. Hyperledger projects provide mentors and the Hyperledger
organization gives some money to participating students.

If you would like mentorship, please apply before the 24th of April 2020.

Looking forward to your applications!

.. toctree::
   :maxdepth: 3
   :caption: Contents:

   installing
   running
   language
   examples
   contributing
