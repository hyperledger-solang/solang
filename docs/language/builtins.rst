Builtin Functions and Variables
===============================

.. _builtins:

The Solidity language has a number of built-in variables and functions which give
access to the chain environment or pre-defined functions. Some of these functions will
be different on different chains.

Block and transaction
_____________________

The functions and variables give access to block properties like block
number and transaction properties like gas used, and value sent.

gasleft() returns (uint64)
++++++++++++++++++++++++++

Returns the amount of gas remaining the current transaction.

.. note::
    ``gasleft()`` is not available on Solana.

    Gasprice is not used on Solana. There is compute budget which may not be
    exceeded, but there is no charge based on compute units used.

blockhash(uint64 block) returns (bytes32)
+++++++++++++++++++++++++++++++++++++++++

Returns the blockhash for a particular block. This not possible for the current
block, or any block except for the most recent 256. Do not use this a source of
randomness unless you know what you are doing.

.. note::
    This function is not available on Solana. There is the
    `recent block hashes account <https://edge.docs.solana.com/developing/runtime-facilities/sysvars#recentblockhashes>`_
    that looks useful at first glance, however it is not usable because:

    - This account is `deprecated <https://github.com/solana-labs/solana/pull/18875>`_.
    - It does not give any slot of block number, so it is not possible to provide a matching
      function signature.

``msg`` properties
++++++++++++++++++

uint128 ``msg.value``
    The amount of value sent with a transaction, or 0 if no value was sent.

bytes ``msg.data``
    The raw ABI encoded arguments passed to the current call.

bytes4 (Polkadot) or bytes8 (Solana) ``msg.sig``
    Function selector (or discriminator for Solana) from the encoded calldata,
    e.g. the first four or eight bytes. This might be 0 if no function selector was present.
    In Ethereum, constructor calls do not have function selectors but in Polkadot they do.
    On Solana, selectors are called discriminators.

address ``msg.sender``
    The sender of the current call. This is either the address of the contract
    that called the current contract, or the address that started the transaction
    if it called the current contract directly.

``tx`` properties
+++++++++++++++++

.. _gasprice:

uint128 ``tx.gasprice``
    The price of one unit of gas. This field cannot be used on Polkadot,
    see the warning box below.

.. note::
    ``tx.gasprice`` is not available on Solana.

    gasprice is not used on Solana. There is compute budget which may not be
    exceeded, but there is no charge based on compute units used.

uint128 ``tx.gasprice(uint64 gas)``
    The total price of `gas` units of gas.

.. warning::
    On Polkadot, the cost of one gas unit may not be an exact whole round value. In fact,
    if the gas price is less than 1 it may round down to 0, giving the incorrect appearance gas is free.
    Therefore, avoid the ``tx.gasprice`` member in favour of the function ``tx.gasprice(uint64 gas)``.

    To avoid rounding errors, pass the total amount of gas into ``tx.gasprice(uint64 gas)`` rather than
    doing arithmetic on the result. As an example, **replace** this bad example:

    .. code-block:: solidity

        // BAD example
        uint128 cost = num_items * tx.gasprice(gas_per_item);

    with:

    .. code-block:: solidity

        uint128 cost = tx.gasprice(num_items * gas_per_item);

    Note this function is not available on the Ethereum Foundation Solidity compiler.

address ``tx.origin``
    The address that started this transaction. Not available on Polkadot or Solana.

AccountInfo[] ``tx.accounts``
    Only available on Solana. See :ref:`account_info`. Here is an example:

.. include:: ../examples/solana/accountinfo.sol
  :code: solidity

``block`` properties
++++++++++++++++++++++

Some block properties are always available:

uint64 ``block.number``
    The current block number.

uint64 ``block.timestamp``
    The time in unix epoch, i.e. seconds since the beginning of 1970.

Do not use either of these two fields as a source of randomness unless you know what
you are doing.

The other block properties depend on which chain is being used.

.. note::
    Solana requires the `clock account <https://edge.docs.solana.com/developing/runtime-facilities/sysvars#clock>`_
    to present in the account for the instruction to use any of the ``block`` fields.

    On Solana, ``block.number`` gives the slot number rather than the block height.
    For processing, you want to use the slot rather the block height. Slots
    include empty blocks, which do not count towards the block height.

Solana
~~~~~~

uint64 ``block.slot``
    The current slot. This is an alias for ``block.number``.

Polkadot
~~~~~~~~

uint128 ``block.minimum_deposit``
    The minimum amonut needed to create a contract. This does not include
    storage rent.

Ethereum
~~~~~~~~

uint64 ``block.gaslimit``
    The current block gas limit.

address payable ``block.coinbase``
    The current block miner's address.

uint256 ``block.difficulty``
    The current block's difficulty.


Error handling
______________

assert(bool)
++++++++++++

Assert takes a boolean argument. If that evaluates to false, execution is aborted.

.. include:: ../examples/revert.sol
  :code: solidity

revert() or revert(string)
++++++++++++++++++++++++++

revert aborts execution of the current contract, and returns to the caller. revert()
can be called with no arguments, or a single `string` argument, which is called the
`ReasonCode`. This function can be called at any point, either in a constructor or
a function.

If the caller is another contract, it can use the `ReasonCode` in a :ref:`try-catch`
statement.

.. include:: ../examples/assert.sol
  :code: solidity

require(bool) or require(bool, string)
++++++++++++++++++++++++++++++++++++++

This function is used to check that a condition holds true, or abort execution otherwise. So,
if the first `bool` argument is `true`, this function does nothing, however
if the `bool` arguments is `false`, then execution is aborted. There is an optional second
`string` argument which is called the `ReasonCode`, which can be used by the caller
to identify what the problem is.

.. include:: ../examples/require.sol
  :code: solidity

ABI encoding and decoding
_________________________

The ABI encoding depends on the target being compiled for. Polkadot uses the
`SCALE Codec <https://docs.substrate.io/reference/scale-codec/>`_.

abi.decode(bytes, (*type-list*))
++++++++++++++++++++++++++++++++

This function decodes the first argument and returns the decoded fields. *type-list* is a comma-separated
list of types. If multiple values are decoded, then a destructure statement must be used.

.. code-block:: solidity

    uint64 foo = abi.decode(bar, (uint64));

.. code-block:: solidity

    (uint64 foo1, bool foo2) = abi.decode(bar, (uint64, bool));

If the arguments cannot be decoded, contract execution will abort. This can happen if the encoded
length is too short, for example.


abi.encode(...)
+++++++++++++++

ABI encodes the arguments to bytes. Any number of arguments can be provided.

.. code-block:: solidity

    uint16 x = 241;
    bytes foo = abi.encode(x);

On Polkadot, foo will be ``hex"f100"``. On Ethereum this will be ``hex"00000000000000000000000000000000000000000000000000000000000000f1"``.

abi.encodeWithSelector(selector, ...)
++++++++++++++++++++++++++++++++++++++++++++

ABI encodes the arguments with the function selector, which is known as the discriminator on Solana.
After the selector, any number of arguments can be provided.

.. code-block:: solidity

    // An eight-byte selector (discriminator) is exclusive for Solana.
    // On Polkadot, the selector contains four bytes. hex"01020304" is an example.
    bytes foo = abi.encodeWithSelector(hex"0102030405060708", uint16(0xff00));

On Solana, foo will be ``hex"080706050403020100ff"``. In addition, a discriminator for a Solidity function on Solana
are the first eight bytes of the sha-256 hash of its name converted to camel case and preceded
by the prefix ``global:``, as the following:

.. code-block:: solidity

    bytes8 discriminator = bytes8(sha256(bytes("global:myFunctionName")));

abi.encodeWithSignature(string signature, ...)
++++++++++++++++++++++++++++++++++++++++++++++

ABI encodes the arguments with the hash of the signature. After the signature, any number of arguments
can be provided.

On Polkadot, the signature is the name of the function followed by its arguments, for example:

.. code-block:: solidity

    bytes foo = abi.encodeWithSignature("foo_bar(uint64)", uint64(257));

``foo`` will be ``hex"e934aa71_0101_0000__0000_0000"``.  This is equivalent to ``abi.encodeWithSelector(bytes4(keccak256("test2(uint64)")), ...)``.

On Solana, the signature is known as the discriminator image. It is the function name without any arguments,
converted to camel case, and preceded by the prefix ``global:``.
For example, if you had the function ``foo_bar(uint64)``, the discriminator image would be ``global:fooBar``.

.. code-block:: solidity

    bytes foo = abi.encodeWithSignature("global:fooBar", uint64(257));

This builtin is equivalent to
``abi.encodeWithSelector(bytes8(sha256(bytes("global:fooBar"))), ...)`` for Solana.

abi.encodePacked(...)
+++++++++++++++++++++

ABI encodes the arguments to bytes. Any number of arguments can be provided. The packed encoding only
encodes the raw data, not the lengths of strings and arrays. For example, when encoding ``string`` only the string
bytes will be encoded, not the length. It is not possible to decode packed encoding.

.. code-block:: solidity

    bytes foo = abi.encodePacked(uint16(0xff00), "ABCD");

On Polkadot, foo will be ``hex"00ff41424344"``. On Ethereum this will be ``hex"ff0041424344"``.

abi.encodeCall(function, ...)
+++++++++++++++++++++++++++++

ABI encodes the function call to the function which should be specified as ``ContractName.FunctionName``. The arguments
are cast and checked against the function specified as the first argument. The arguments must be in a tuple, e.g.
``(a, b, c)``. If there is a single argument no tuple is required.

.. include:: ../examples/abi_encode_call.sol
  :code: solidity

Hash
++++

Only available on Polkadot, it represents the ``Hash`` type from ``ink_primitives`` via user type definition.
Its underlying type is ``bytes32``, but it will be reported correctly as the ``Hash`` type in the metadata.

.. include:: ../examples/polkadot/hash_type.sol
  :code: solidity

chain_extension(uint32 ID, bytes input) returns (uint32, bytes)
+++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++

Only available on Polkadot. Call the chain extension with the given ``ID`` and ``input`` data.
Returns the return value from the chain extension and the output data.

This function is a low level interface.
The caller is responsible for encoding the input and decoding the output correctly.
We expect parachain authors to write their own higher level libraries on top.

.. warning::
    This function calls the runtime API `call_chain_extension <https://docs.rs/pallet-contracts/latest/pallet_contracts/api_doc/trait.Version0.html#tymethod.call_chain_extension>`_.

	It assumes that the implementation of the chain extension
	    - reads the input from the ``input_ptr`` parameter, used as a buffer pointer
	    - writes potential output into the buffer found at the ``output_ptr`` pointer
	    - respects the output buffer length in ``output_len_ptr`` to prevent OOB writes. The output buffer is 16KB in size.
	    - writes the amount of bytes written to ``output_ptr`` into the buffer at ``output_len_ptr``

	Unlike with other runtime API calls, the contracts pallet can not guarantee this behaviour.
	Instead, it's specific to the targeted chain runtime. Hence, when using this builtin,
	you must be sure that the implementation being called underneath is compatible.

The following example demonstrates the usage of this builtin function.
It shows how the chain extension example from the `ink! documentation <https://use.ink/macros-attributes/chain-extension/>`_
looks like in a solidity contract:

.. include:: ../examples/polkadot/call_chain_extension.sol
  :code: solidity

is_contract(address AccountId) returns (bool)
+++++++++++++++++++++++++++++++++++++++++++++

Only available on Polkadot. Checks whether the given address is a contract address.

caller_is_root() returns (bool)
+++++++++++++++++++++++++++++++

Only available on Polkadot. Returns true if the caller of the contract is `root <https://docs.substrate.io/build/origins/>`_.

set_code_hash(uint8[32] hash) returns (uint32)
++++++++++++++++++++++++++++++++++++++++++++++

Only available on Polkadot. Replace the contract's code with the code corresponding to ``hash``.
Assumes that the new code was already uploaded, otherwise the operation fails.
A return value of 0 indicates success; a return value of 7 indicates that there was no corresponding code found.

.. note::

    This is a low level function. We strongly advise consulting the underlying
    `API documentation <https://docs.rs/pallet-contracts/latest/pallet_contracts/api_doc/trait.Version0.html#tymethod.set_code_hash>`_
    to obtain a full understanding of its implications.

This functionality is intended to be used for implementing upgradeable contracts.
Pitfalls generally applying to writing
`upgradeable contracts <https://docs.openzeppelin.com/upgrades-plugins/1.x/writing-upgradeable>`_
must be considered whenever using this builtin function, most notably:

* The contract must safeguard access to this functionality, so that it is only callable by priviledged users.
* The code you are upgrading to must be
  `storage compatible <https://docs.openzeppelin.com/upgrades-plugins/1.x/proxies#storage-collisions-between-implementation-versions>`_
  with the existing code.
* Constructors and any other initializers, including initial storage value definitions, won't be executed.

Cryptography
____________

keccak256(bytes)
++++++++++++++++

This returns the ``bytes32`` keccak256 hash of the bytes.

ripemd160(bytes)
++++++++++++++++

This returns the ``bytes20`` ripemd160 hash of the bytes.

sha256(bytes)
+++++++++++++

This returns the ``bytes32`` sha256 hash of the bytes.

blake2_128(bytes)
+++++++++++++++++

This returns the ``bytes16`` blake2_128 hash of the bytes.

.. note::

    This function is only available on Polkadot.

blake2_256(bytes)
+++++++++++++++++

This returns the ``bytes32`` blake2_256 hash of the bytes.

.. note::

    This function is only available on Polkadot.

signatureVerify(address public_key, bytes message, bytes signature)
+++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++

Verify the ed25519 signature given the public key, message, and signature. This
function returns ``true`` if the signature matches, ``false`` otherwise.

The transactions which executes this function, needs an
`ed25519 program <https://edge.docs.solana.com/developing/runtime-facilities/programs#ed25519-program>`_
instruction with matching public key, message, and signature.
In order to examine the instruction, the
`instructions sysvar <https://edge.docs.solana.com/developing/runtime-facilities/sysvars#instructions>`_
needs be in the accounts for the Solidity instruction as well.

.. note::

   This function is only available on Solana.

Mathematical
____________

addmod(uint x, uint y, uint, k) returns (uint)
++++++++++++++++++++++++++++++++++++++++++++++

Add x to y, and then divides by k. x + y will not overflow.

mulmod(uint x, uint y, uint, k) returns (uint)
++++++++++++++++++++++++++++++++++++++++++++++

Multiply x with y, and then divides by k. x * y will not overflow.

Encoding and decoding values from bytes buffer
______________________________________________

The ``abi.encode()`` and friends functions do not allow you to write or read data
from an arbitrary offset, so the Solang dialect has the following functions. These
methods are available on a ``bytes`` type.

These functions are inspired by the `node buffer api <https://nodejs.org/api/buffer.html>`_.

.. code-block:: solidity

    contract c {
        function f() public returns (bytes) {
            bytes data = new bytes(10);
            data.writeUint32LE(102, 0);
            data.writeUint64LE(0xdeadcafe, 4);
            return data;
        }

        function g(bytes data) public returns (uint64) {
            return data.readUint64LE(1);
        }
    }

readInt8(uint32 offset)
+++++++++++++++++++++++

Read a signed ``int8`` from the specified offset.

readInt16LE(uint32 offset)
++++++++++++++++++++++++++

Read a signed ``int16`` from the specified offset in little endian order.

readInt32LE(uint32 offset)
++++++++++++++++++++++++++

Read a signed ``int32`` from the specified offset in little endian order.

readInt64LE(uint32 offset)
++++++++++++++++++++++++++

Read a signed ``int64`` from the specified offset in little endian order.

readInt128LE(uint32 offset)
+++++++++++++++++++++++++++

Read a signed ``int128`` from the specified offset in little endian order.

readInt256LE(uint32 offset)
+++++++++++++++++++++++++++

Read a signed ``int256`` from the specified offset in little endian order.

readUint16LE(uint32 offset)
+++++++++++++++++++++++++++

Read an unsigned ``uint16`` from the specified offset in little endian order.

readUint32LE(uint32 offset)
+++++++++++++++++++++++++++

Read an unsigned ``uint32`` from the specified offset in little endian order.

readUint64LE(uint32 offset)
+++++++++++++++++++++++++++

Read an unsigned ``uint64`` from the specified offset in little endian order.

readUint128LE(uint32 offset)
++++++++++++++++++++++++++++

Read an unsigned ``uint128`` from the specified offset in little endian order.

readUint256LE(uint32 offset)
++++++++++++++++++++++++++++

Read an unsigned ``uint256`` from the specified offset in little endian order.

readAddress(uint32 offset)
++++++++++++++++++++++++++

Read an ``address`` from the specified offset.

writeInt8(int8 value, uint32 offset)
++++++++++++++++++++++++++++++++++++

Write a signed ``int8`` to the specified offset.

writeInt16LE(int16 value, uint32 offset)
++++++++++++++++++++++++++++++++++++++++

Write a signed ``int16`` to the specified offset in little endian order.

writeInt32LE(int32 value, uint32 offset)
++++++++++++++++++++++++++++++++++++++++

Write a signed ``int32`` to the specified offset in little endian order.

writeInt64LE(int64 value, uint32 offset)
++++++++++++++++++++++++++++++++++++++++

Write a signed ``int64`` to the specified offset in little endian order.

writeInt128LE(int128 value, uint32 offset)
++++++++++++++++++++++++++++++++++++++++++

Write a signed ``int128`` to the specified offset in little endian order.

writeInt256LE(int256 value, uint32 offset)
++++++++++++++++++++++++++++++++++++++++++

Write a signed ``int256`` to the specified offset in little endian order.

writeUint16LE(uint16 value, uint32 offset)
++++++++++++++++++++++++++++++++++++++++++

Write an unsigned ``uint16`` to the specified offset in little endian order.

writeUint32LE(uint32 value, uint32 offset)
++++++++++++++++++++++++++++++++++++++++++

Write an unsigned ``uint32`` to the specified offset in little endian order.

writeUint64LE(uint64 value, uint32 offset)
++++++++++++++++++++++++++++++++++++++++++

Write an unsigned ``uint64`` to the specified offset in little endian order.

writeUint128LE(uint128 value, uint32 offset)
++++++++++++++++++++++++++++++++++++++++++++

Write an unsigned ``uint128`` to the specified offset in little endian order.

writeUint256LE(uint256 value, uint32 offset)
++++++++++++++++++++++++++++++++++++++++++++

Write an unsigned ``uint256`` to the specified offset in little endian order.

writeAddress(address value, uint32 offset)
++++++++++++++++++++++++++++++++++++++++++

Write an ``address`` to the specified offset.

writeString(string value, uint32 offset)
++++++++++++++++++++++++++++++++++++++++++++

Write the characters of a ``string`` to the specified offset. This function does not
write the length of the string to the buffer.

writeBytes(bytes value, uint32 offset)
++++++++++++++++++++++++++++++++++++++++++

Write the bytes of a Solidity dynamic bytes type ``bytes`` to the specified offset.
This function does not write the length of the byte array to the buffer.


Miscellaneous
_____________

.. _print_function:

print(string)
+++++++++++++

print() takes a string argument.

.. include:: ../examples/print.sol
  :code: solidity

.. note::

  print() is not available with the Ethereum Foundation Solidity compiler.

  When using Polkadot, this function is only available on development chains.
  If you use this function on a production chain, the contract will fail to load.

.. _selfdestruct:

selfdestruct(address payable recipient)
+++++++++++++++++++++++++++++++++++++++

The ``selfdestruct()`` function causes the current contract to be deleted, and any
remaining balance to be sent to `recipient`. This functions does not return, as the
contract no longer exists.

.. note::
    This function does not exist on Solana.

String formatting using ``"{}".format()``
+++++++++++++++++++++++++++++++++++++++++

Sometimes it is useful to convert an integer to a string, e.g. for debugging purposes. There is
a format builtin function for this, which is a method on string literals. Each ``{}`` in the
string will be replaced with the value of an argument to format().

.. code-block:: solidity

    function foo(int arg1, bool arg2) public {
        print("foo entry arg1:{} arg2:{}".format(arg1, arg2));
    }

Assuming `arg1` is 5355 and `arg2` is true, the output to the log will be ``foo entry arg1:5355 arg2:true``.

The types accepted by format are ``bool``, ``uint``, ``int`` (any size, e.g. ``int128`` or ``uint64``), ``address``,
``bytes`` (fixed and dynamic), and ``string``. Enums are also supported, but will print the ordinal value
of the enum. The ``uint`` and ``int`` types can have a format specifier. This allows you to convert to
hexadecimal ``{:x}`` or binary ``{:b}``, rather than decimals. No other types
have a format specifier. To include a literal ``{`` or ``}``, replace it with ``{{`` or ``}}``.


.. code-block:: solidity

    function foo(int arg1, uint arg2) public {
        // print arg1 in hex, and arg2 in binary
        print("foo entry {{arg1:{:x},arg2:{:b}}}".format(arg1, arg2));
    }

Assuming `arg1` is 512 and `arg2` is 196, the output to the log will be ``foo entry {arg1:0x200,arg2:0b11000100}``.

.. warning::

    Each time you call the ``format()`` some specialized code is generated, to format the string at
    runtime. This requires loops and so on to do the conversion.

    When formatting integers in to decimals, types larger than 64 bits require expensive division.
    Be mindful this will increase the gas cost. Larger values will incur a higher gas cost.
    Alternatively, use a hexadecimal ``{:x}`` format specifier to reduce the cost.


extendTtl(uint32 threshold, uint32 extend_to) 
+++++++++++++++++++++++++++++++++++++++++++++++++++++++

The ``extendTtl()`` method allows extending the time-to-live (TTL) of a contract storage entry.

If the entry's TTL is below threshold ledgers, this function updates ``live_until_ledger_seq`` such that TTL equals ``extend_to``. The TTL is defined as:

.. math::

TTL = live_until_ledger_seq - current_ledger


.. note:: This method is only available on the Soroban target

.. code-block:: solidity

    /// Extends the TTL for the `count` persistent key to 5000 ledgers
    /// if the current TTL is smaller than 1000 ledgers
    function extend_ttl() public view returns (int64) {
        return count.extendTtl(1000, 5000);
    }



For more details on managing contract data TTLs in Soroban, refer to the docs for `TTL <https://developers.stellar.org/docs/build/smart-contracts/getting-started/storing-data#managing-contract-data-ttls-with-extend_ttl>`_.

extendInstanceTtl(uint32 threshold, uint32 extend_to)
+++++++++++++++++++++++++++++++++++++++++++++++++++++++

The extendInstanceTtl() function extends the time-to-live (TTL) of contract instance storage.

If the TTL for the current contract instance and code (if applicable) is below threshold ledgers, this function extends ``live_until_ledger_seq`` such that TTL equals ``extend_to``.

.. note:: This is a global function, not a method, and is only available on the Soroban target

.. code-block:: solidity

    /// Extends the TTL for the contract instance storage to 10000 ledgers
    /// if the current TTL is smaller than 2000 ledgers
    function extendInstanceTtl() public view returns (int64) {
        return extendInstanceTtl(2000, 10000);
    }
