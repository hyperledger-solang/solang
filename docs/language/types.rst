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

    literal 300 is too large to fit into type 'uint8'

.. tip::

  When using integers, whenever possible use the ``int64``, ``int32`` or ``uint64``,
  ``uint32`` types.

  The Solidity language has its origins for the Ethereum Virtual Machine (EVM), which has
  support for 256 bit arithmetic. Most common CPUs like x86_64 do not implement arithmetic
  for such large types, and any EVM virtual machine implementation has to do bigint
  calculations, which are expensive.

  WebAssembly or BPF do not support this. As a result that Solang has to emulate larger types with
  many instructions, resulting in larger contract code and higher gas cost.

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
the target being compiled for. On ewasm, an address is 20 bytes. Solana and Substrate have an address
length of 32 bytes. The format of an address literal depends on what target you are building for. On ewasm,
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

Substrate or Solana addresses are base58 encoded, not hexadecimal. An address literal can be specified with
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

    Substrate can be compiled with a different type for Address. If your substrate has a different
    length for address, you can specify ``--address-length`` on the command line.

Enums
_____

Solidity enums types need to have a definition which lists the possible values it can hold. An enum
has a type name, and a list of unique values. Enum types can used in public functions, but the value
is represented as a ``uint8`` in the ABI. Enum are limited to 256 values.

.. code-block:: javascript

  contract enum_example {
      enum Weekday { Monday, Tuesday, Wednesday, Thursday, Friday, Saturday, Sunday }

      function is_weekend(Weekday day) public pure returns (bool) {
          return (day == Weekday.Saturday || day == Weekday.Sunday);
      }
  }

An enum can be converted to and from integer, but this requires an explicit cast. The value of an enum
is numbered from 0, like in C and Rust.

If enum is declared in another contract, the type can be refered to with `contractname.typename`. The
individual enum values are `contractname.typename.value`. The enum declaration does not have to appear
in a contract, in which case it can be used without the contract name prefix.

.. code-block:: javascript

    enum planets { Mercury, Venus, Earth, Mars, Jupiter, Saturn, Uranus, Neptune }

    contract timeofday {
        enum time { Night, Day, Dawn, Dusk }
    }

    contract stargazing {
        function look_for(timeofday.time when) public returns (planets[]) {
            if (when == timeofday.time.Dawn || when == timeofday.time.Dusk) {
                planets[] x = new planets[](2);
                x[0] = planets.Mercury;
                x[1] = planets.Venus;
                return x;
            } else if (when == timeofday.time.Night) {
                planets[] x = new planets[](5);
                x[0] = planets.Mars;
                x[1] = planets.Jupiter;
                x[2] = planets.Saturn;
                x[3] = planets.Uranus;
                x[4] = planets.Neptune;
                return x;
            } else {
                planets[] x = new planets[](1);
                x[0] = planets.Earth;
                return x;
            }
        }
    }

Struct Type
___________

A struct is composite type of several other types. This is used to group related items together.

.. code-block:: solidity

  contract deck {
      enum suit { club, diamonds, hearts, spades }
      enum value { two, three, four, five, six, seven, eight, nine, ten, jack, queen, king, ace }
      struct card {
          value v;
          suit s;
      }

      function score(card c) public returns (uint32 score) {
          if (c.s == suit.hearts) {
              if (c.v == value.ace) {
                  score = 14;
              }
              if (c.v == value.king) {
                  score = 13;
              }
              if (c.v == value.queen) {
                  score = 12;
              }
              if (c.v == value.jack) {
                  score = 11;
              }
          }
          // all others score 0
      }
  }

A struct has one or more fields, each with a unique name. Structs can be function arguments and return
values. Structs can contain other structs. There is a struct literal syntax to create a struct with
all the fields set.

.. code-block:: solidity

  contract deck {
      enum suit { club, diamonds, hearts, spades }
      enum value { two, three, four, five, six, seven, eight, nine, ten, jack, queen, king, ace }
      struct card {
          value v;
          suit s;
      }

      card card1 = card(value.two, suit.club);
      card card2 = card({s: suit.club, v: value.two});

      // This function does a lot of copying
      function set_card1(card c) public returns (card previous) {
          previous = card1;
          card1 = c;
      }
  }

The two contract storage variables ``card1`` and ``card2`` have initializers using struct literals. Struct
literals can either set fields by their position, or field name. In either syntax, all the fields must
be specified. When specifying structs fields by position, the order of the fields must match with the
struct definition. When fields are specified by name, the order is not important.

Struct definitions from other contracts can be used, by referring to them with the `contractname.`
prefix. Struct definitions can appear outside of contract definitions, in which case they can be used
in any contract without the prefix.

.. code-block:: solidity

    struct user {
        string name;
        bool active;
    }

    contract auth {
        function authenticate(string name, db.users storage users) public returns (bool) {
            // ...
        }
    }

    contract db {
        struct users {
            user[] field1;
            int32 count;
        }
    }

The `users` struct contains an array of `user`, which is another struct. The `users` struct is
defined in contract `db`, and can be used in another contract with the type name `db.users`. Astute
readers may have noticed that the `db.users` struct is used before it is declared. In Solidity,
types can be always be used before their declaration, or before they are imported.

Structs can be contract storage variables. Structs in contract storage can be assigned to structs
in memory and vice versa, like in the *set_card1()* function. Copying structs between storage
and memory is expensive; code has to be generated for each field and executed.

- The function argument ``c`` has to ABI decoded (1 copy + decoding overhead)
- The ``card1`` has to load from contract storage (1 copy + contract storage overhead)
- The ``c`` has to be stored into contract storage (1 copy + contract storage overhead)
- The ``previous`` struct has to ABI encoded (1 copy + encoding overhead)

Note that struct variables are references. When contract struct variables or normal struct variables
are passed around, just the memory address or storage slot is passed around internally. This makes
it very cheap, but it does mean that if a called function modifies the struct, then this is
visible in the caller as well.

.. code-block:: solidity

  contract foo {
      struct bar {
          bytes32 f1;
          bytes32 f2;
          bytes32 f3;
          bytes32 f4;
      }

      function f(bar b) public {
          b.f4 = "foobar";
      }

      function example() public {
          bar bar1;

          // bar1 is passed by reference; just its pointer is passed
          f(bar1);

          assert(bar1.f4 == "foobar");
      }
  }

.. note::

  In the Ethereum Foundation Solidity compiler, you need to add ``pragma experimental ABIEncoderV2;``
  to use structs as return values or function arguments in public functions. The default ABI encoder
  of Solang can handle structs, so there is no need for this pragma. The Solang compiler ignores
  this pragma if present.

Fixed Length Arrays
___________________

Arrays can be declared by adding [length] to the type name, where length is a
constant expression. Any type can be made into an array, including arrays themselves (also
known as arrays of arrays). For example:

.. code-block:: solidity

    contract foo {
        /// In a vote with 11 voters, do the ayes have it?
        function f(bool[11] votes) public pure returns (bool) {
            uint32 i;
            uint32 ayes = 0;

            for (i=0; i<votes.length; i++) {
                if (votes[i]) {
                    ayes += 1;
                }
            }

            // votes.length is odd; integer truncation means that 11 / 2 = 5
            return ayes > votes.length / 2;
        }
    }

Note the length of the array can be read with the ``.length`` member. The length is readonly.
Arrays can be initialized with an array literal. For example:

.. code-block:: solidity

    contract primes {
        function primenumber(uint32 n) public pure returns (uint64) {
            uint64[10] primes = [ 2, 3, 5, 7, 11, 13, 17, 19, 23, 29 ];

            return primes[n];
        }
    }

Any array subscript which is out of bounds (either an negative array index, or an index past the
last element) will cause a runtime exception. In this example, calling ``primenumber(10)`` will
fail; the first prime number is indexed by 0, and the last by 9.

Arrays are passed by reference. If you modify the array in another function, those changes will
be reflected in the current function. For example:

.. code-block:: solidity

    contract reference {
        function set_2(int8[4] a) pure private {
            a[2] = 102;
        }

        function foo() private {
            int8[4] val = [ int8(1), 2, 3, 4 ];

            set_2(val);

            // val was passed by reference, so was modified
            assert(val[2] == 102);
        }
    }

.. note::

  In Solidity, an fixed array of 32 bytes (or smaller) can be declared as ``bytes32`` or
  ``uint8[32]``. In the Ethereum ABI encoding, an ``int8[32]`` is encoded using
  32 × 32 = 1024 bytes. This is because the Ethereum ABI encoding pads each primitive to
  32 bytes. However, since ``bytes32`` is a primitive in itself, this will only be 32
  bytes when ABI encoded.

  In Substrate, the `SCALE <https://substrate.dev/docs/en/knowledgebase/advanced/codec>`_
  encoding uses 32 bytes for both types.

Dynamic Length Arrays
_____________________

Dynamic length arrays are useful for when you do not know in advance how long your arrays
will need to be. They are declared by adding ``[]`` to your type. How they can be used depends
on whether they are contract storage variables or stored in memory.

Memory dynamic arrays must be allocated with ``new`` before they can be used. The ``new``
expression requires a single unsigned integer argument. The length can be read using
``length`` member variable. Once created, the length of the array cannot be changed.

.. code-block:: solidity

    contract dynamicarray {
        function test(uint32 size) public {
            int64[] memory a = new int64[](size);

            for (uint32 i = 0; i < size; i++) {
                a[i] = 1 << i;
            }

            assert(a.length == size);
        }
    }


.. note::

    There is experimental support for `push()` and `pop()` on memory arrays.

Storage dynamic memory arrays do not have to be allocated. By default, they have a
length of zero and elements can be added and removed using the ``push()`` and ``pop()``
methods.

.. code-block:: javascript

    contract s {
        int64[] a;

        function test() public {
            // push takes a single argument with the item to be added
            a.push(128);
            // push with no arguments adds 0
            a.push();
            // now we have two elements in our array, 128 and 0
            assert(a.length == 2);
            a[0] |= 64;
            // pop removes the last element
            a.pop();
            // you can assign the return value of pop
            int64 v = a.pop();
            assert(v == 192);
        }
    }

Calling the method ``pop()`` on an empty array is an error and contract execution will abort,
just like when you access an element beyond the end of an array.

``push()`` without any arguments returns a storage reference. This is only available for types
that support storage references (see below).

.. code-block:: solidity

    contract example {
        struct user {
            address who;
            uint32 hitcount;
        }
        s[] foo;

        function test() public {
            // foo.push() creates an empty entry and returns a reference to it
            user storage x = foo.push();

            x.who = address(1);
            x.hitcount = 1;
        }
    }

Depending on the array element, ``pop()`` can be costly. It has to first copy the element to
memory, and then clear storage.

String
______

Strings can be initialized with a string literal or a hex literal. Strings can be concatenated and
compared, and formatted using `.format()`; no other operations are allowed on strings.

.. code-block:: solidity

    contract example {
        function test1(string s) public returns (bool) {
            string str = "Hello, " + s + "!";

            return (str == "Hello, World!");
        }

        function test2(string s, int64 n) public returns (string res) {
            res = "Hello, {}! #{}".format(s, n);
        }
    }

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

.. code-block:: solidity

    contract b {
        function test() public {
            bytes a = hex"0000_00fa";
            bytes b = new bytes(4);

            b[3] = hex"fa";

            assert(a == b);
        }
    }

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

- it has to be in contract storage, not memory
- they are not iterable
- the key cannot be a ``struct``, array, or another mapping.

Mappings are declared with ``mapping(keytype => valuetype)``, for example:

.. code-block:: solidity

    contract b {
        struct user {
            bool exists;
            address addr;
        }
        mapping(string => user) users;

        function add(string name, address addr) public {
            // assigning to a storage variable creates a reference
            user storage s = users[name];

            s.exists = true;
            s.addr = addr;
        }

        function get(string name) public view returns (bool, address) {
            // assigning to a memory variable creates a copy
            user s = users[name];

            return (s.exists, s.addr);
        }

        function rm(string name) public {
            delete users[name];
        }
    }

.. tip::

  When assigning multiple members in a struct in a mapping, it is better to create
  a storage variable as a reference to the struct, and then assign to the reference.
  The add() function above could have been written as:

  .. code-block:: solidity

    function add(string name, address addr) public {
        s[name].exists = true;
        s[name].addr = addr;
    }

  Here the storage slot for struct is calculated twice, which includes an expensive
  keccak256 calculation.

If you access a non-existing field on a mapping, all the fields will read as zero. So, it
is common practise to have a boolean field called ``exists``. Since mappings are not iterable,
it is not possible to do a ``delete`` on an mapping, but an entry can be deleted.

.. note::

  Solidity takes the keccak 256 hash of the key and the storage slot, and simply uses that
  to find the entry. There are no hash collision chains. This scheme is simple and avoids
  `"hash flooding" <https://www.securityweek.com/hash-table-collision-attacks-could-trigger-ddos-massive-scale>`_
  attacks where the attacker chooses data which hashes to the same hash
  collision chain, making the hash table very slow; it will behave like a linked list.

  In order to implement mappings in memory, a new scheme must be found which avoids this
  attack. Usually this is done with `SipHash <https://en.wikipedia.org/wiki/SipHash>`_, but
  this cannot be used in smart contracts since there is no place to store secrets. Collision
  chains are needed since memory has a much smaller address space than the 256 bit storage
  slots.

  Any suggestions for solving this are very welcome!

Contract Types
______________

In Solidity, other smart contracts can be called and created. So, there is a type to hold the
address of a contract. This is in fact simply the address of the contract, with some syntax
sugar for calling functions it.

A contract can be created with the new statement, followed by the name of the contract. The
arguments to the constructor must be provided.

.. code-block:: solidity

    contract child {
        function announce() public {
            print("Greetings from child contract");
        }
    }

    contract creator {
        function test() public {
            child c = new child();

            c.announce();
        }
    }

Since child does not have a constructor, no arguments are needed for the new statement. The variable
`c` of the contract `child` type, which simply holds its address. Functions can be called on
this type. The contract type can be cast to and from address, provided an explicit cast is used.

The expression ``this`` evaluates to the current contract, which can be cast to ``address`` or
``address payable``.

.. code-block:: solidity

    contract example {
        function get_address() public returns (address) {
            return address(this);
        }
    }

Function Types
______________

Function types are references to functions. You can use function types to pass functions
for callbacks, for example. Function types come in two flavours, ``internal`` and ``external``.
An internal function is a reference to a function in the same contract or one of its base contracts.
An external function is a reference to a public or external function on any contract.

When declaring a function type, you must specify the parameters types, return types, mutability,
and whether it is external or internal. The parameters or return types cannot have names.

.. code-block:: solidity

    contract ft {
        function test() public {
            // reference to an internal function with two argments, returning bool
            // with the default mutability (i.e. cannot be payable)
            function(int32, bool) internal returns (bool) x;

            // the local function func1 can be assigned to this type; mutability
            // can be more restrictive than the type.
            x = func1;

            // now you can call func1 via the x
            bool res = x(102, false);

            // reference to an internal function with no return values, must be pure
            function(int32 arg1, bool arg2) internal pure y;

            // Does not compile: wrong number of return types and mutability
            // is not compatible.
            y = func1;
        }

        function func1(int32 arg, bool arg2) view internal returns (bool) {
            return false;
        }
    }

If the ``internal`` or ``external`` keyword is omitted, the type defaults to internal.

Just like any other type, a function type can be a function argument, function return type, or a
contract storage variable. Internal function types cannot be used in public functions parameters or
return types.

An external function type is a reference to a function in a particular contract. It stores the address of
the contract, and the function selector. An internal function type only stores the function reference. When
assigning a value to an external function selector, the contract and function must be specified, by using
a function on particular contract instance.

.. code-block:: solidity

    contract ft {
        function test(paffling p) public {
            // this.callback can be used as an external function type value
            p.set_callback(this.callback);
        }

        function callback(int32 count, string foo) public {
            // ...
        }
    }

    contract paffling {
        // the first visibility "external" is for the function type, the second "internal" is
        // for the callback variables
        function(int32, string) external internal callback;

        function set_callback(function(int32, string) external c) public {
            callback = c;
        }

        function piffle() public {
            callback(1, "paffled");
        }
    }


Storage References
__________________

Parameters, return types, and variables can be declared storage references by adding
``storage`` after the type name. This means that the variable holds a references to a
particular contract storage variable.

.. code-block:: solidity

    contract felix {
        enum Felines { None, Lynx, Felis, Puma, Catopuma };
        Felines[100] group_a;
        Felines[100] group_b;


        function count_pumas(Felines[100] storage cats) private returns (uint32)
    {
            uint32 count = 0;
            uint32 i = 0;

            for (i = 0; i < cats.length; i++) {
                if (cats[i] == Felines.Puma) {
                    ++count;
                }
            }

            return count;
        }

        function all_pumas() public returns (uint32) {
            Felines[100] storage ref = group_a;

            uint32 total = count_pumas(ref);

            ref = group_b;

            total += count_pumas(ref);

            return total;
        }
    }

Functions which have either storage parameter or return types cannot be public; when a function
is called via the ABI encoder/decoder, it is not possible to pass references, just values.
However it is possible to use storage reference variables in public functions, as
demonstrated in function all_pumas().

User Defined Types
__________________

A user defined type is a new type which simply wraps an existing primitive type. First, a new type
is declared with the ``type`` syntax. The name of the type can now be used anywhere where a type
is used, for example in function arguments or return values.

.. code-block:: solidity

    type Value is uint128;

    function inc_and_wrap(int128 v) returns (Value) {
        return Value.wrap(v + 1);
    }

    function dec_and_unwrap(Value v) returns (uint128) {
        return Value.unwrap(v) - 1;
    }

Note that the wrapped value ``Value v`` cannot be used in any type of arithmetic or comparision. It needs to
be unwrapped before it can be used.
