.. _language-status:

Brief Language status
=====================

The Solidity language supported by Solang aims to be compatible with the latest
`Ethereum Foundation Solidity Compiler <https://github.com/ethereum/solidity/>`_,
version 0.8 with some small exceptions.

.. note::

  Where differences exist between different targets or the Ethereum Foundation Solidity
  compiler, this is noted in boxes like these.

As with any new project, bugs are possible. Please report any issues you may find to github.

Differences:

- libraries are always statically linked into the contract code
- Solang generates WebAssembly or Solana SBF rather than EVM.
- Packed encoded uses little endian encoding, as WASM and SBF are little endian
  virtual machines.

Unique features to Solang:

- Solang can target different blockchains and some features depending on the target.
  For example, Polkadot uses a different ABI encoding and allows constructors
  to be named.
- Events can be declared outside of contracts
- Base contracts can be declared in any order
- There is a ``print()`` function for debugging
- Strings can be formatted with python style format string, which is useful for debugging: ``print("x = {}".format(x));``
- Ethereum style address literals like ``0xE0f5206BBD039e7b0592d8918820024e2a7437b9`` are
  not supported on Polkadot or Solana, but are supported for EVM.
- On Polkadot and Solana, base58 style encoded address literals like
  ``address"5GBWmgdFAMqm8ZgAHGobqDqX6tjLxJhv53ygjNtaaAn3sjeZ"`` are supported, but
  not with EVM.
- On Solana, there is special builtin import file called ``'solana'`` available.
- On Polkadot, there is special builtin import file called ``'polkadot'`` available.
- Different blockchains offer different builtins. See the :ref:`builtins documentation <builtins>`.
- There are many more differences, which are noted throughout the documentation.