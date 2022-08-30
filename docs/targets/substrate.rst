Parity Substrate
================

Solang works with Parity Substrate 3.0. Note that for recent Substrate Version, cross-contract calls as well as using `address`
type as function argument or return values are not supported. We are currently working on fixing any regressions.

The Parity Substrate has the following differences to Ethereum Solidity:

- The address type is 32 bytes, not 20 bytes. This is what Substrate calls an "account"
- An address literal has to be specified using the ``address"5GBWmgdFAMqm8ZgAHGobqDqX6tjLxJhv53ygjNtaaAn3sjeZ"`` syntax
- ABI encoding and decoding is done using the `SCALE <https://docs.substrate.io/reference/scale-codec/>`_ encoding
- Multiple constructors are allowed, and can be overloaded
- There is no ``ecrecover()`` builtin function, or any other function to recover or verify cryptographic signatures at runtime
- Only functions called via rpc may return values; when calling a function in a transaction, the return values cannot be accessed
- An `assert()`, `require()`, or `revert()` executes the wasm unreachable instruction. The reason code is lost

There is an solidity example which can be found in the
`examples <https://github.com/hyperledger/solang/tree/main/examples>`_
directory. Write this to flipper.sol and run:

.. code-block:: bash

  solang compile --target substrate flipper.sol

Now you should have a file called ``flipper.contract``. The file contains both the ABI and contract wasm.
It can be used directly in the
`Contracts UI <https://contracts-ui.substrate.io/>`_, as if the contract was written in ink!.
