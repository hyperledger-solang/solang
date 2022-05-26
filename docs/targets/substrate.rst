Parity Substrate
________________

Solang works with Parity Substrate 2.0 or later.

The Parity Substrate has the following differences to Ethereum Solidity:

- The address type is 32 bytes, not 20 bytes. This is what Substrate calls an "account"
- An address literal has to be specified using the ``address"5GBWmgdFAMqm8ZgAHGobqDqX6tjLxJhv53ygjNtaaAn3sjeZ"`` syntax
- ABI encoding and decoding is done using the `SCALE <https://substrate.dev/docs/en/knowledgebase/advanced/codec>`_ encoding
- Multiple constructors are allowed, and can be overloaded
- There is no ``ecrecover()`` builtin function, or any other function to recover or verify cryptographic signatures at runtime
- Only functions called via rpc may return values; when calling a function in a transaction, the return values cannot be accessed
- An `assert()`, `require()`, or `revert()` executes the wasm unreachable instruction. The reason code is lost

There is an solidity example which can be found in the
`examples <https://github.com/hyperledger-labs/solang/tree/main/examples>`_
directory. Write this to flipper.sol and run:

.. code-block:: bash

  solang --target substrate flipper.sol

Now you should have a file called ``flipper.contract``. The file contains both the ABI and contract wasm.
It can be used directly in the
`Polkadot UI <https://substrate.dev/substrate-contracts-workshop/#/0/deploy-contract>`_, as if the contract was written in ink!.
