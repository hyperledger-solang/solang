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

bytes4 ``msg.sig``
    Function selector from the ABI encoded calldata, e.g. the first four bytes. This
    might be 0 if no function selector was present. In Ethereum, constructor calls do not
    have function selectors but in Parity Substrate they do.

address ``msg.sender``
    The sender of the current call. This is either the address of the contract
    that called the current contract, or the address that started the transaction
    if it called the current contract directly.

``tx`` properties
+++++++++++++++++

.. _gasprice:

uint128 ``tx.gasprice``
    The price of one unit of gas. This field cannot be used on Parity Substrate,
    see the warning box below.

.. note::
    ``tx.gasprice`` is not available on Solana.

    gasprice is not used on Solana. There is compute budget which may not be
    exceeded, but there is no charge based on compute units used.

uint128 ``tx.gasprice(uint64 gas)``
    The total price of `gas` units of gas.

.. warning::
    On Parity Substrate, the cost of one gas unit may not be an exact whole round value. In fact,
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
    The address that started this transaction. Not available on Parity Substrate or Solana.

AccountInfo[] ``tx.accounts``
    Only available on Solana. See :ref:`account_info`. Here is an example:

.. code-block:: solidity

    import {AccountInfo} from 'solana';

    contract SplToken {
       function get_token_account(address token) internal view returns (AccountInfo) {
               for (uint64 i = 0; i < tx.accounts.length; i++) {
                       AccountInfo ai = tx.accounts[i];
                       if (ai.key == token) {
                               return ai;
                       }
               }

               revert("token not found");
       }

        function total_supply(address token) public view returns (uint64) {
                AccountInfo account = get_token_account(token);

                return account.data.readUint64LE(33);
        }
    }

address ``tx.program_id``
    The address or account of the currently executing program. Only available on
    Solana.

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

Parity Substrate
~~~~~~~~~~~~~~~~

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


.. code-block:: solidity

    contract c {
        constructor(int x) {
            assert(x > 0);
        }
    }

revert() or revert(string)
++++++++++++++++++++++++++

revert aborts execution of the current contract, and returns to the caller. revert()
can be called with no arguments, or a single `string` argument, which is called the
`ReasonCode`. This function can be called at any point, either in a constructor or
a function.

If the caller is another contract, it can use the `ReasonCode` in a :ref:`try-catch`
statement.

.. code-block:: solidity

    contract x {
        constructor(address foobar) {
            if (a == address(0)) {
                revert("foobar must a valid address");
            }
        }
    }

require(bool) or require(bool, string)
++++++++++++++++++++++++++++++++++++++

This function is used to check that a condition holds true, or abort execution otherwise. So,
if the first `bool` argument is `true`, this function does nothing, however
if the `bool` arguments is `false`, then execution is aborted. There is an optional second
`string` argument which is called the `ReasonCode`, which can be used by the caller
to identify what the problem is.

.. code-block:: solidity

    contract x {
        constructor(address foobar) {
            require(foobar != address(0), "foobar must a valid address");
        }
    }


ABI encoding and decoding
_________________________

The ABI encoding depends on the target being compiled for. Substrate uses the
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

On Substrate, foo will be ``hex"f100"``. On Ethereum this will be ``hex"00000000000000000000000000000000000000000000000000000000000000f1"``.

abi.encodeWithSelector(bytes4 selector, ...)
++++++++++++++++++++++++++++++++++++++++++++

ABI encodes the arguments with the function selector first. After the selector, any number of arguments
can be provided.

.. code-block:: solidity

    bytes foo = abi.encodeWithSelector(hex"01020304", uint16(0xff00), "ABCD");

On Substrate, foo will be ``hex"0403020100ff"``. On Ethereum this will be ``hex"01020304000000000000000000000000000000000000000000000000000000000000ff00"``.

abi.encodeWithSignature(string signature, ...)
++++++++++++++++++++++++++++++++++++++++++++++

ABI encodes the arguments with the ``bytes4`` hash of the signature. After the signature, any number of arguments
can be provided. This is equivalent to ``abi.encodeWithSignature(bytes4(keccak256(signature)), ...)``.

.. code-block:: solidity

    bytes foo = abi.encodeWithSignature("test2(uint64)", uint64(257));

On Substrate, foo will be ``hex"296dacf0_0101_0000__0000_0000"``. On Ethereum this will be ``hex"296dacf0_0000000000000000000000000000000000000000000000000000000000000101"``.

abi.encodePacked(...)
+++++++++++++++++++++

ABI encodes the arguments to bytes. Any number of arguments can be provided. The packed encoding only
encodes the raw data, not the lengths of strings and arrays. For example, when encoding ``string`` only the string
bytes will be encoded, not the length. It is not possible to decode packed encoding.

.. code-block:: solidity

    bytes foo = abi.encodePacked(uint16(0xff00), "ABCD");

On Substrate, foo will be ``hex"00ff41424344"``. On Ethereum this will be ``hex"ff0041424344"``.

abi.encodeCall(function, ...)
+++++++++++++++++++++++++++++

ABI encodes the function call to the function which should be specified as ``ContractName.FunctionName``. The arguments
are cast and checked against the function specified as the first argument.

.. code-block:: solidity

    contract c {
        function f1() public {
            bytes foo = abi.encodeCall(c.bar, 102, true);
        }

        function bar(int a, bool b) public {}
    }

Hash
++++

Only available on Substrate, it represents the ``Hash`` type from ``ink_primitives`` via user type definition.
Its underlying type is ``bytes32``, but it will be reported correctly as the ``Hash`` type in the metadata.

.. code-block:: solidity

    import 'substrate';

    contract c {
        bytes32 current;

        function set(Hash h) public returns (Hash) {
            current = Hash.unwrap(h);
            return Hash.wrap(current);
        }
    }

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

    This function is only available on Parity Substrate.

blake2_256(bytes)
+++++++++++++++++

This returns the ``bytes32`` blake2_256 hash of the bytes.

.. note::

    This function is only available on Parity Substrate.

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

print(string)
+++++++++++++

print() takes a string argument.

.. code-block:: solidity

    contract c {
        constructor() {
            print("Hello, world!");
        }
    }

.. note::

  print() is not available with the Ethereum Foundation Solidity compiler.

  When using Substrate, this function is only available on development chains.
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
