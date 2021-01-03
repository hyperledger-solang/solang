.. _status:

Status
======

Solidity Language Status
------------------------

Solang is source compatible with the
`Ethereum Foundation Solidity Compiler <https://github.com/ethereum/solidity/>`_
version 0.7, with some caveats. Many language features have only recently been
implemented, and have many unit tests. As with any new project, bugs are possible.
Please report any issues you may find to github.

Differences:

- ``immutable`` is not supported. Note this is impossible to implement on any than chain other than Ethereum; this is purely an ethereum feature
- libraries are always statically linked into the contract code
- Solang generates WebAssembly or BPF rather than EVM. This means that the ``assembly {}``
  statement using EVM instructions is not supported

Unique features to Solang:

- Solang can target different blockchains and some features depending on the target.
  For example, Parity Substrate uses a different ABI encoding and allows constructors
  to be overloaded.
- Events can be declared outside of contracts
- Base contracts can be declared in any order
- There is a ``print()`` function for debugging
- Strings can be formatted with python style format string, which is useful for debugging: ``print("x = {}".format(x));``

Target Status
-------------

Parity Substrate
________________

Solang works with Parity Substrate 2.0. This target is the most mature and has received the most testing so far.

Solana
______

Solang has a new target for `Solana <https://www.solana.com/>`_. This is in early stages right now, however it is
under active development.

ewasm
_____

ewasm has been tested with `Hyperledger Burrow <https://github.com/hyperledger/burrow>`_.
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