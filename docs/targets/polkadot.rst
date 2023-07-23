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
`examples <https://github.com/hyperledger/solang/tree/main/examples>`_
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

When a contract reverts, the returned error data is what
`EVM would return <https://docs.soliditylang.org/en/v0.8.20/control-structures.html#panic-via-assert-and-error-via-require>`_.
``assert()``, ``require()``, or ``revert()`` will revert the contract execution, where the revert reason 
is supplied as the contracts output (if any). Solidity contracts can also revert with a `Panic`.
Uncaught exceptions from calling and instantiating contracts or transferring funds will be bubbled 
up back to the caller.

The metadata contains all error variants that the contract knows about in the ``lang_error`` field.
The 4 bytes `selector` of the error data can be seen as the enum discriminator. However, because 
SCALE encoding does not allow discriminators larger than 1 byte, the hex-encoded error selector 
is provided as the enum variant name in the metadata (the selector could also be calculated by
reconstructing and hashing the error signature based on the enum variant types).

.. note::

    Solidity knows about `Error`, `Panic` and 
    `custom errors <https://docs.soliditylang.org/en/latest/abi-spec.html#errors>`_.
    Solang does not yet support custom errors. For now, only `Error` (selector of `0x08c379a0`) 
    and `Panic` (selector of `4e487b71`) will be returned and occur in the metadata.

The general process of decoding the output data for Solang Solidity contracts is as follows:

1. If the revert flag is **not** set, the contract didn't revert and the output should be encoded as specified in the message spec.
2. The compiler version must be solang > 0.3.1, or the error data can't be decoded (check the ``compiler`` field in the contract metadata).
3. If the output length is smaller than 4 bytes, the error data can't be decoded (contracts may return empty error data, for example if ``revert()`` without arguments is used).
4. If the first 4 bytes of the output do **not** match any of the selectors found in ``lang_error``, the error can't be decoded.
5. **Skip** the selector (first 4 bytes) and decode the remaining data according to the corresponding variant type found in `lang_error`.

.. warning::

    Do not trust the error data.

    Solidity contracts do bubble up uncaught errors. This can lead to situations where the 
    contract reverts with error data unknown to the contracts. Examples of this include 
    bubbling up custom error data from the callee or error data from an ``ink!`` contract.
