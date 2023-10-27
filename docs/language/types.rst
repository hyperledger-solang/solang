Types
=====

The following primitive types are supported.

Boolean Type
____________

``bool``
  This represents a single value which can be either ``true`` or ``false``.

Integer Types
_____________

``uint``
  This represents a single unsigned integer of 256 bits wide. Values can be for example
  ``0``, ``102``, ``0xdeadcafe``, or ``1000_000_000_000_000``.

``uint64``, ``uint32``, ``uint16``, ``uint8``
  These represent shorter single unsigned integers of the given width. These widths are
  most efficient and should be used whenever possible.

``uintN``
  These represent shorter single unsigned integers of width ``N``. ``N`` can be anything
  between 8 and 256 bits and a multiple of 8, e.g. ``uint24``.

``int``
  This represents a single signed integer of 256 bits wide. Values can be for example
  ``-102``, ``0``, ``102`` or ``-0xdead_cafe``.

``int64``, ``int32``, ``int16``, ``int8``
  These represent shorter single signed integers of the given width. These widths are
  most efficient and should be used whenever possible.

``intN``
  These represent shorter single signed integers of width ``N``. ``N`` can be anything
  between 8 and 256 bits and a multiple of 8, e.g. ``int128``.

Underscores ``_`` are allowed in numbers, as long as the number does not start with
an underscore.  ``1_000`` is allowed but ``_1000`` is not. Similarly
``0xffff_0000`` is fine, but ``0x_f`` is not.

Scientific notation is supported, e.g. ``1e6`` is one million. Only integer values
are supported.

Assigning values which cannot fit into the type gives a compiler error. For example::

    uint8 foo = 300;

The largest value an ``uint8`` can hold is (2 :superscript:`8`) - 1 = 255. So, the compiler says:

.. code-block:: none

    value 300 does not fit into type uint8

.. tip::

  When using integers, whenever possible use the ``int64``, ``int32`` or ``uint64``,
  ``uint32`` types.

  The Solidity language has its origins for the Ethereum Virtual Machine (EVM), which has
  support for 256 bit arithmetic. Most common CPUs like x86_64 do not implement arithmetic
  for such large types, and any EVM virtual machine implementation has to do bigint
  calculations, which are expensive.

  WebAssembly or Solana SBF do not support this. As a result that Solang has to emulate larger types with
  many instructions, resulting in larger contract code and higher gas cost or compute units.

Fixed Length byte arrays
________________________

Solidity has a primitive type unique to the language. It is a fixed-length byte array of 1 to 32
bytes, declared with *bytes* followed by the array length, for example:
``bytes32``, ``bytes24``, ``bytes8``, or ``bytes1``. ``byte`` is an alias for ``byte1``, so
``byte`` is an array of 1 element. The arrays can be initialized with either a hex string ``hex"414243"``,
or a text string ``"ABC"``, or a hex value ``0x414243``.

.. code-block:: solidity

  bytes4 foo = "ABCD";
  bytes4 bar = hex"41_42_43_44";

The ascii value for ``A`` is 41 in hexadecimal. So, in this case, foo and bar
are initialized to the same value. Underscores are allowed in hex strings; they exist to aid
readability. If the string is shorter than the type, it is padded with zeros. For example:

.. code-block:: solidity

  bytes6 foo = "AB" "CD";
  bytes5 bar = hex"41";

String literals can be concatenated like they can in C or C++. Here the types are longer than
the initializers; this means they are padded at the end with zeros. foo will contain the following
bytes in hexadecimal ``41 42 43 44 00 00`` and bar will be ``41 00 00 00 00``.

These types can be used with all the bitwise operators, ``~``, ``|``, ``&``, ``^``, ``<<``, and
``>>``. When these operators are used, the type behaves like an unsigned integer type. In this case
think the type not as an array but as a long number. For example, it is possible to shift by one bit:

.. code-block:: solidity

  bytes2 foo = hex"0101" << 1;
  // foo is 02 02

Since this is an array type, it is possible to read array elements too. They are indexed from zero.
It is not permitted to set array elements; the value of a bytesN type can only be changed
by setting the entire array value.

.. code-block:: solidity

  bytes6 wake_code = "heotymeo";
  bytes1 second_letter = wake_code[1]; // second_letter is "e"

The length can be read using the ``.length`` member variable. Since this is a fixed size array, this
is always the length of the type itself.

.. code-block:: solidity

  bytes32 hash;
  assert(hash.length == 32);
  byte b;
  assert(b.length == 1);

Address and Address Payable Type
________________________________

The ``address`` type holds the address of an account. The length of an ``address`` type depends on
the target being compiled for. On EVM, an address is 20 bytes. Solana and Polkadot have an address
length of 32 bytes. The format of an address literal depends on what target you are building for. On EVM,
ethereum addresses can be specified with a particular hexadecimal number.

.. code-block:: solidity

  address foo = 0xE9430d8C01C4E4Bb33E44fd7748942085D82fC91;

The hexadecimal string should be 40 hexadecimal characters, and not contain any underscores.
The capitalization, i.e. whether ``a`` to ``f`` values are capitalized, is important.
It is defined in
`EIP-55 <https://github.com/ethereum/EIPs/blob/master/EIPS/eip-55.md>`_. For example,
when compiling:

.. code-block:: solidity

  address foo = 0xe9430d8C01C4E4Bb33E44fd7748942085D82fC91;

Since the hexadecimal string is 40 characters without underscores, and the string does
not match the EIP-55 encoding, the compiler will refused to compile this. To make this
a regular hexadecimal number, not an address literal, add some leading zeros or some underscores.
In order to fix the address literal, copy the address literal from the compiler error message:

.. code-block:: none

  error: address literal has incorrect checksum, expected ‘0xE9430d8C01C4E4Bb33E44fd7748942085D82fC91’

Polkadot or Solana addresses are base58 encoded, not hexadecimal. An address literal can be specified with
the special syntax ``address"<account>"``.

.. code-block:: solidity

    address foo = address"5GBWmgdFAMqm8ZgAHGobqDqX6tjLxJhv53ygjNtaaAn3sjeZ";

An address can be payable or not. An payable address can used with the
:ref:`.send() and .transfer() functions <send_transfer>`, and
:ref:`selfdestruct` function. A non-payable address or contract can be cast to an ``address payable``
using the ``payable()`` cast, like so:

.. code-block:: solidity

    address payable addr = payable(this);

``address`` cannot be used in any arithmetic or bitwise operations. However, it can be cast to and from
bytes types and integer types. The ``==`` and ``!=`` operators work for comparing two address types.

.. code-block:: solidity

  address foo = address(0);

.. note::
    The type name ``address payable`` cannot be used as a cast in the Ethereum Foundation Solidity compiler,
    and the cast should be declared ``payable`` instead. This is
    `apparently due to a limitation in their parser <https://github.com/ethereum/solidity/pull/4926#discussion_r216586365>`_.
    Solang's generated parser has no such limitation and allows ``address payable`` to be used as a cast,
    but allows ``payable`` to be used as a cast well, for compatibility reasons.

.. note::

    Polkadot can be compiled with a different type for Address. If your target runtime has a different
    length for address, you can specify ``--address-length`` on the command line.

Enums
_____

Solidity enums types need to have a definition which lists the possible values it can hold. An enum
has a type name, and a list of unique values. Enum types can used in public functions, but the value
is represented as a ``uint8`` in the ABI. Enum are limited to 256 values.

.. include:: ../examples/enum_type.sol
  :code: solidity

An enum can be converted to and from integer, but this requires an explicit cast. The value of an enum
is numbered from 0, like in C and Rust.

If enum is declared in another contract, the type can be referred to with `contractname.typename`. The
individual enum values are `contractname.typename.value`. The enum declaration does not have to appear
in a contract, in which case it can be used without the contract name prefix.

.. include:: ../examples/enum_type_external.sol
  :code: solidity

Struct Type
___________

A struct is composite type of several other types. This is used to group related items together.

.. include:: ../examples/struct_type.sol
  :code: solidity

A struct has one or more fields, each with a unique name. Structs can be function arguments and return
values. Structs can contain other structs. There is a struct literal syntax to create a struct with
all the fields set.

.. include:: ../examples/struct_type_arguments.sol
  :code: solidity

The two contract storage variables ``card1`` and ``card2`` have initializers using struct literals. Struct
literals can either set fields by their position, or field name. In either syntax, all the fields must
be specified. When specifying structs fields by position, the order of the fields must match with the
struct definition. When fields are specified by name, the order is not important.

Struct definitions from other contracts can be used, by referring to them with the `contractname.`
prefix. Struct definitions can appear outside of contract definitions, in which case they can be used
in any contract without the prefix.

.. include:: ../examples/struct_type_arguments_external.sol
  :code: solidity

The `users` struct contains an array of `user`, which is another struct. The `users` struct is
defined in contract `db`, and can be used in another contract with the type name `db.users`.
Notice that the `db.users` struct type is used in the function `authenticate` before it is declared. In Solidity,
types can be always be used before their declaration, or even before the ``import`` directive.

Structs can be contract storage variables. Structs in contract storage can be assigned to structs
in memory and vice versa, like in the *set_card1()* function. Copying structs between storage
and memory is expensive; code has to be generated and executed for each field. In the *set_card1* function,
the following is done:

- The function argument ``c`` has to ABI decoded (1 copy + decoding overhead)
- The ``card1`` has to load from contract storage (1 copy + contract storage overhead)
- The ``c`` has to be stored into contract storage (1 copy + contract storage overhead)
- The ``previous`` struct has to ABI encoded (1 copy + encoding overhead)

Note that struct variables are references. When contract struct variables or normal struct variables
are passed around, just the memory address or storage slot is passed around internally. This makes
it very cheap, but it does mean that if a called function modifies the struct, then this is
visible in the caller as well.

.. include:: ../examples/struct_type_variable_references.sol
  :code: solidity

Fixed Length Arrays
___________________

Arrays can be declared by adding [length] to the type name, where length is a
constant expression. Any type can be made into an array, including arrays themselves (also
known as arrays of arrays). For example:

.. include:: ../examples/array_type_fixed_length.sol
  :code: solidity

Note the length of the array can be read with the ``.length`` member. The length is readonly.
Arrays can be initialized with an array literal. For example:

.. include:: ../examples/array_type_initialized.sol
  :code: solidity

Any array subscript which is out of bounds (either an negative array index, or an index past the
last element) will cause a runtime exception. In this example, calling ``primenumber(10)`` will
fail; the first prime number is indexed by 0, and the last by 9.

Arrays are passed by reference. If you modify the array in another function, those changes will
be reflected in the current function. For example:

.. include:: ../examples/array_type_references.sol
  :code: solidity

On Solang, it is not necessary to cast the first element of the array literal.

.. note::

  In Solidity, an fixed array of 32 bytes (or smaller) can be declared as ``bytes32`` or
  ``uint8[32]``. In the Ethereum ABI encoding, an ``int8[32]`` is encoded using
  32 × 32 = 1024 bytes. This is because the Ethereum ABI encoding pads each primitive to
  32 bytes. However, since ``bytes32`` is a primitive in itself, this will only be 32
  bytes when ABI encoded.

  On Polkadot, the `SCALE <https://docs.substrate.io/reference/scale-codec/>`_
  encoding uses 32 bytes for both types. Similarly, the `borsh encoding <https://borsh.io/>`_
  used on Solana uses 32 bytes for both types.

Dynamic Length Arrays
_____________________

Dynamic length arrays are useful for when you do not know in advance how long your arrays
will need to be. They are declared by adding ``[]`` to your type. How they can be used depends
on whether they are contract storage variables or stored in memory.

Memory dynamic arrays must be allocated with ``new`` before they can be used. The ``new``
expression requires a single unsigned integer argument. The length can be read using
``length`` member variable.

.. include:: ../examples/array_type_dynamic.sol
  :code: solidity

.. note::

    There is experimental support for `push()` and `pop()` on memory arrays.

Storage dynamic memory arrays do not have to be allocated. By default, they have a
length of zero and elements can be added and removed using the ``push()`` and ``pop()``
methods.

.. include:: ../examples/array_type_dynamic_storage.sol
  :code: solidity

Calling the method ``pop()`` on an empty array is an error and contract execution will abort,
just like when accessing an element beyond the end of an array.

``push()`` without any arguments returns a storage reference. This is only available for types
that support storage references (see below).

.. include:: ../examples/array_type_dynamic_push.sol
  :code: solidity

Depending on the array element, ``pop()`` can be costly. It has to first copy the element to
memory, and then clear storage.

String
______

Strings can be initialized with a string literal or a hex literal. Strings can be concatenated and
compared, and formatted using `.format()`; no other operations are allowed on strings.

.. include:: ../examples/string_type.sol
  :code: solidity

Strings can be cast to `bytes`. This cast has no runtime cost, since both types use
the same underlying data structure.

.. note::

    The Ethereum Foundation Solidity compiler does not allow unicode characters in string literals,
    unless it is prefixed with unicode, e.g. ``unicode"€"`` . For compatibility, Solang also
    accepts the unicode prefix. Solang always allows unicode characters in strings.

Dynamic Length Bytes
____________________

The ``bytes`` datatype is a dynamic length array of bytes. It can be created with
the ``new`` operator, or from an string or hex initializer. Unlike the ``string`` type,
it is possible to index the ``bytes`` datatype like an array.

.. include:: ../examples/dynamic_bytes_type.sol
  :code: solidity

If the ``bytes`` variable is a storage variable, there is a ``push()`` and ``pop()``
method available to add and remove bytes from the array. Array elements in a
memory ``bytes`` can be modified, but no elements can be removed or added, in other
words, ``push()`` and ``pop()`` are not available when ``bytes`` is stored in memory.

A ``string`` type can be cast to ``bytes``. This way, the string can be modified or
characters can be read. Note this will access the string by byte, not character, so
any non-ascii characters will need special handling.

An dynamic array of bytes can use the type ``bytes`` or ``byte[]``. The latter
stores each byte in an individual storage slot, while the former stores the
entire string in a single storage slot, when possible. Additionally a ``string``
can be cast to ``bytes`` but not to ``byte[]``.

Mappings
________

Mappings are a dictionary type, or associative arrays. Mappings have a number of
limitations:

- They only work as storage variables
- They are not iterable
- The key cannot be a ``struct``, array, or another mapping.

Mappings are declared with ``mapping(keytype => valuetype)``, for example:

.. include:: ../examples/mapping_type.sol
  :code: solidity

Mappings may have a name for the key or the value, for example:
``mapping(address owner => uint64 balance)``. The names are used in the metadata of
the contract. If the mapping is public, the accessor function will have
named arguments and returns.

.. tip::

  When assigning multiple members in a struct in a mapping, it is better to create
  a storage variable as a reference to the struct, and then assign to the reference.
  The ``add()`` function above can be optimized like the following.

  .. code-block:: solidity

    function add(string name, address addr) public {
        // assigning to a storage variable creates a reference
        user storage s = users[name];

        s.exists = true;
        s.addr = addr;
    }

  Here the storage slot for the struct is calculated only once, avoiding another expensive
  keccak256 calculation.

If you access a non-existing field on a mapping, all the fields will read as zero. It
is common practise to have a boolean field called ``exists``. Since mappings are not iterable,
it is not possible to ``delete`` an entire mapping itself, but individual mapping entries can be deleted.

.. note::

  Solidity on Ethereum and on Polkadot takes the keccak 256 hash of the key and the storage slot, and simply uses that
  to find the entry. Its underlying hash table does not use separate chaining for collision resolution.
  The scheme is simple and avoids `"hash flooding" <https://en.wikipedia.org/wiki/Collision_attack#Hash_flooding>`_
  attacks that utilize hash collisions to exploit the worst-case time complexity for a separately chained
  hash table. When too many collisions exist in a such a data structure,
  it degenerates to a linked list, whose time complexity for searches is O(n).

  In order to implement mappings on Solana's storage, a new scheme must be found to prevent this
  attack. `SipHash <https://en.wikipedia.org/wiki/SipHash>`_ is a hash algorithm that solves the problem,
  but it cannot be used in smart contracts since there is no place to store secrets. Separate
  chaining for collision handling is needed since Solana accounts have a much smaller address
  space than the 256 bit storage slots. Any suggestions for solving this are very welcome!

  SipHash may serve as a way to implement mappings in memory, which would allow them to be local variables in
  functions. Although, a safe alternative to random seeds still needs to be found.

Contract Types
______________

In Solidity, other smart contracts can be called and created. So, there is a type to hold the
address of a contract. This is in fact simply the address of the contract, with some syntax
sugar for calling functions on it.

A contract can be created with the new statement, followed by the name of the contract. The
arguments to the constructor must be provided.

.. include:: ../examples/polkadot/contract_type.sol
  :code: solidity

Since child does not have a constructor, no arguments are needed for the new statement. The variable
`c` of the contract `child` type, which simply holds its address. Functions can be called on
this type. The contract type can be cast to and from address, provided an explicit cast is used.

The expression ``this`` evaluates to the current contract, which can be cast to ``address`` or
``address payable``.

.. include:: ../examples/contract_type_cast_address.sol
  :code: solidity

.. _contracts_not_types:

.. note::
    On Solana, contracts cannot exist as types, so contracts cannot be function parameters, function returns
    or variables. Contracts on Solana are deployed to a defined address, which is often known during compile time,
    so there is no need to hold that address as a variable underneath a contract type.


Function Types
______________

Function types are references to functions. You can use function types to pass functions
for callbacks, for example. Function types come in two flavours, ``internal`` and ``external``.
An internal function is a reference to a function in the same contract or one of its base contracts.
An external function is a reference to a public or external function on any contract.

When declaring a function type, you must specify the parameters types, return types, mutability,
and whether it is external or internal. The parameters or return types cannot have names.

.. include:: ../examples/function_type.sol
  :code: solidity

If the ``internal`` or ``external`` keyword is omitted, the type defaults to internal.

Just like any other type, a function type can be a function argument, function return type, or a
contract storage variable. Internal function types cannot be used in public functions parameters or
return types.

An external function type is a reference to a function in a particular contract. It stores the address of
the contract, and the function selector. An internal function type only stores the function reference. When
assigning a value to an external function selector, the contract and function must be specified, by using
a function on particular contract instance.

.. tabs::

    .. group-tab:: Polkadot

        .. include:: ../examples/polkadot/function_type_callback.sol
            :code: solidity


    .. group-tab:: Solana

        .. include:: ../examples/solana/function_type_callback.sol
            :code: solidity


On Solana, external calls from variables of type external functions require the ``accounts`` call argument. The
compiler cannot determine the accounts such a function needs, so it does not automatically generate the
``AccountsMeta`` array.

.. code-block:: solidity

    function test(function(int32, string) external myFunc) public {
        myFunc{accounts: []}(24, "accounts");
    }

Storage References
__________________

Parameters, return types, and variables can be declared storage references by adding
``storage`` after the type name. This means that the variable holds a references to a
particular contract storage variable.

.. include:: ../examples/storage_ref_type.sol
  :code: solidity

Functions which have either storage parameter or return types cannot be public; when a function
is called via the ABI encoder/decoder, it is not possible to pass references, just values.
However it is possible to use storage reference variables in public functions, as
demonstrated in function all_pumas().

.. _user_defined_types:

User Defined Types
__________________

A user defined type is a new type which simply wraps an existing primitive type. First, a new type
is declared with the ``type`` syntax. The name of the type can now be used anywhere where a type
is used, for example in function arguments or return values.

.. include:: ../examples/user_defined_type.sol
  :code: solidity

Note that the wrapped value ``Value v`` cannot be used in any type of arithmetic or comparison. It needs to
be unwrapped before it can be used.

User Defined Types can be used with :ref:`user defined operators <user_defined_operators>`.
