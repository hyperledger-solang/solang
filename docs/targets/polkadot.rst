Polkadot
========

Solang works on Polkadot Parachains integrating a recent version of the ``contracts`` pallets.
Solidity flavored for the Polkadot target has the following differences to Ethereum Solidity:

- The address type is 32 bytes, not 20 bytes. This is what Substrate calls an "account".
- An address literal has to be specified using the ``address"5GBWmgdFAMqm8ZgAHGobqDqX6tjLxJhv53ygjNtaaAn3sjeZ"`` syntax
- ABI encoding and decoding is done using the `SCALE <https://docs.substrate.io/reference/scale-codec/>`_ encoding
- Constructors can be named. Constructors with no name will be called ``new`` in the generated metadata.
- There is no ``ecrecover()`` builtin function, or any other function to recover or verify cryptographic signatures at runtime
- Only functions called via rpc may return values; when calling a function in a transaction, the return values cannot be accessed

There is a solidity example which can be found in the
`examples <https://github.com/hyperledger-solang/solang/tree/main/examples>`_
directory. Write this to flipper.sol and run:

.. code-block:: bash

  solang compile --target polkadot flipper.sol

Now you should have a file called ``flipper.contract``. The file contains both the ABI and contract wasm.
It can be used directly in the
`Contracts UI <https://contracts-ui.substrate.io/>`_, as if the contract was written in ink!.

Builtin Imports
________________

Some builtin functionality is only available after importing. The following types
can be imported via the special import file ``polkadot``.

.. code-block:: solidity

    import {Hash} from 'polkadot';
    import {chain_extension} from 'polkadot';

Note that ``{Hash}`` can be omitted, renamed or imported via
import object.

.. code-block:: solidity

    // Now Hash will be known as InkHash
    import {Hash as InkHash} from 'polkadot';

.. note::

    The import file ``polkadot`` is only available when compiling for the Polkadot target.

Call Flags
__________

The Substrate contracts pallet knows several 
`flags <https://github.com/paritytech/substrate/blob/6e0059a416a5768e58765a49b33c21920c0b0eb9/frame/contracts/src/wasm/runtime.rs#L392>`_ 
that can be used when calling other contracts.

Solang allows a ``flags`` call argument of type ``uint32`` in the ``address.call()`` function to set desired flags.
By default (if this argument is unset), no flag will be set.

The following example shows how call flags can be used:

.. include:: ../examples/polkadot/call_flags.sol
  :code: solidity


Reverts and error data decoding
_______________________________

When a contract reverts, the returned error data is what the
`EVM would return <https://docs.soliditylang.org/en/v0.8.20/control-structures.html#panic-via-assert-and-error-via-require>`_.
``assert()``, ``require()``, or ``revert()`` will revert the contract execution, where the revert reason (if any) is 
encoded as ``Error(string)`` and provided in the execution output. Solidity contracts can also revert with a ``Panic(uint256)`` 
(please refer to the 
`Ethereum Solidity language documentation <https://docs.soliditylang.org/en/v0.8.20/control-structures.html#panic-via-assert-and-error-via-require>`_
for more information about when ``Panic`` might be returned).
Uncaught exceptions from calling and instantiating contracts or transferring funds will be bubbled 
up back to the caller.

.. note::

    Solidity knows about ``Error``, ``Panic`` and custom errors. Please, also refer to the
    `Ethereum Solidity documentation <https://docs.soliditylang.org/en/latest/abi-spec.html#errors>`_,
    for more information.

The metadata contains all error variants that the contract `knows` about in the ``lang_error`` field.

.. warning::

    Never trust the error data.

    Solidity contracts do bubble up uncaught errors. This can lead to situations where the 
    contract reverts with error data unknown to the contracts. Examples of this include 
    bubbling up custom error data from the callee or error data from an ``ink!`` contract.

The 4 bytes selector of the error data can be seen as the enum discriminator or index. However, 
because SCALE encoding does not allow index larger than 1 byte, the hex-encoded error selector 
is provided as the path of the error variant type in the metadata.

In the following example, the ``Panic`` variant of ``lang_error`` is of type ``10``, which looks like this:

.. code-block:: json

    {
      "id": 10,
      "type": {
        "def": {
          "composite": {
            "fields": [
              {
                "type": 9
              }
            ]
          }
        },
        "path": [
          "0x4e487b71"
        ]
      }
    }

From this follows that error data matching the ``Panic`` selector of `0x4e487b71` can be decoded 
according to type ``10`` (where the decoder must exclude the first 4 selector bytes).

The general process of decoding the output data of Solang Solidity contracts is as follows:

1. The compiler of the contract must be Solang (check the ``compiler`` field in the contract metadata).
2. If the revert flag is **not** set, the contract didn't revert and the output should be decoded as specified in the message spec.
3. If the output length is smaller than 4 bytes, the error data can't be decoded (contracts may return empty error data, for example if ``revert()`` without arguments is used).
4. If the first 4 bytes of the output do **not** match any of the selectors found in ``lang_error``, the error can't be decoded.
5. **Skip** the selector (first 4 bytes) and decode the remaining data according to the matching type found in `lang_error`.
