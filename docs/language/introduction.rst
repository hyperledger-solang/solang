Brief Language status
=====================

The Solidity language supported by Solang aims to be compatible with the latest
`Ethereum Foundation Solidity Compiler <https://github.com/ethereum/solidity/>`_,
version 0.7 with some caveats.

.. note::

  Where differences exist between different targets or the Ethereum Foundation Solidity
  compiler, this is noted in boxes like these.

As with any new project, bugs are possible. Please report any issues you may find to github.

Differences:

- libraries are always statically linked into the contract code
- Solang generates WebAssembly or BPF rather than EVM.

Unique features to Solang:

- Solang can target different blockchains and some features depending on the target.
  For example, Parity Substrate uses a different ABI encoding and allows constructors
  to be overloaded.
- Events can be declared outside of contracts
- Base contracts can be declared in any order
- There is a ``print()`` function for debugging
- Strings can be formatted with python style format string, which is useful for debugging: ``print("x = {}".format(x));``
