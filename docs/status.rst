Status
======

Solang is a very young project, and does not support all the features that the
`Ethereum Foundation Solidity Compiler <https://github.com/ethereum/solidity/>`_
supports. The majority of the features is supported and the project is ready
for use.

Testing Status
--------------

Many language features have only recently been implemented, and have many unit
tests. However, as with any new project, bugs are possible. Please report any
issues you may find to github.

.. _language_status:

Solidity Language completeness
------------------------------

Solang wants to be compatible with the latest version of
`Ethereum Foundation Solidity Compiler <https://github.com/ethereum/solidity/>`_. The
project is under active development, and new language features are being added
on a continuous basis.

Missing features:

- ``immutable`` is not supported. Note this is impossible to implement on Parity Substrate or Hyperledger Sawtooth; this is purely an ethereum feature
- libraries functions are always statically linked into the contract wasm
- function types are not implemented yet
- Solang generates WebAssembly rather than EVM. This means that the ``assembly {}``
  statement using EVM instructions is not supported
- Defining functions outside of contracts (solc 0.7.1)
- Getting the selector of the a function using the ``.selector`` syntax
- Calling parent contract via ``super``

Unique features to Solang:

- Solang can target different blockchains and some features depending on the target.
  For example, Parity Substrate uses a different ABI encoding and allows constructors
  to be overloaded.
- Events can be declared outside of contracts
- Base contracts can be declared in any order
- There is a ``print()`` function for debugging.

Target Status
-------------

Parity Substrate
________________

This target is the most mature and has received the most testing so far. Solang has
been tested with Parity Substrate has been tested on 2.0-rc4. 2.0-rc5 and later
are known not to work, due to changes in the contracts host interface.

ewasm
_____

ewasm has been tested with `Hyperledeger Burrow <https://github.com/hyperledger/burrow>`_.
Please use the latest master version of burrow, as ewasm support is still maturing in Burrow.

Some language features have not been fully implemnented yet on ewasm:

- The built in function ``abi.encode()``, ``abi.encodeWithSelector()``, ``abi.encodeWithSignature()``, and ``abi.encodePacked()``
- Contract storage variables types ``string`` and ``bytes`` are not implemented

Hyperledger Sawtooth
____________________

This is merely a proof-of-concept target, and has seen very little testing. On sawtooth,
many Solidity concepts are impossible to implement:

- Return values from contract calls
- Calling other contracts
- Value transfers
- Instantiating contracts