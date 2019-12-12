Solidity Language
=================

The Solidity language support by Solang is compatible with the
`Ethereum Foundation Solidity Compiler <https://github.com/ethereum/solidity/>`_ with
these caveats:

- At this point solang is very much a work in progress so not at all features
  are supported yet.

- Solang can target different blockchains and some features depending on the target.
  For example, Parity Substrate uses a different ABI encoding and allows constructors
  to be overloaded.

- Solang generates WebAssembly rather than EVM. This means that the ``assembly {}``
  using EVM instructions is not supported.

.. note::

  Where differences exist between different targets or the Ethereum Foundation Solidity
  compiler, this is noted in boxes like these.

Types
-----

The following primitive types are supported:

``bool``
  This represents a single value which can be either ``true`` or ``false``

``uint``
  This represents a single unsigned integer of 256 bits wide. Values can be for example
  ``0``, ``102``, ``0xdeadcafe``, or ``1000_000_000_000_000``.

``uint64``, ``uint32``, ``uint16``, ``uint8``
  These represent shorter single unsigned integers of the given width. These widths are
  most efficient in WebAssembly and should be used whenever possible.

``uintN``
  These represent shorter single unsigned integers of width ``N``. ``N`` can be anything
  between 8 and 256 bits.

``int``
  This represents a single signed integer of 256 bits wide. Values can be for example
  ``-102``, ``0``, ``102`` or ``-0xdead_cafe``.

``int64``, ``uint32``, ``uint16``, ``uint8``
  These represent shorter single signed integers of the given width. These widths are
  most efficient in WebAssembly and should be used whenever possible.

``intN``
  These represent shorter single signed integers of width ``N``. ``N`` can be anything
  between 8 and 256 bits.

Underscores ``_`` are allowed in numbers, as long as the number does not start with
an underscore. This means that ``1_000`` is allowed but ``_1000`` is not. Similarly
``0xffff_0000`` is fine, but ``0x_f`` is not.

Assigning values which cannot fit into the type gives a compiler error. For example::

    uint8 foo = 300;

The largest value an ``uint8`` can hold is (2^8) - 1 = 255. So, the compiler says::

    implicit conversion would truncate from uint16 to uint8

.. note::

  The Ethereum Foundation Solidity compiler supports additional data types: address,
  bytes and string. These will be implemented in Solang in early 2020.

.. tip::

  When using integers, whenever possible use the ``int64``, ``int32`` or ``uint64``,
  ``uint32`` types.

  The Solidity language has its origins in the Ethereum Virtual Machine (EVM), which has
  support for 256 bit registers. Most common CPUs like x86_64 do not implement arithmetic
  for such large types, and the EVM virtual machine itself has to do bigint calculations, which
  are costly. This means that EVM instructions with gas cost of 1 can be very expensive in
  real CPU cost.

  WebAssembly does not support this. This means that Solang has to emulate larger types with
  multiple WebAssembly instructions, resulting in larger contract code and higher gas cost.
  As a result, gas cost approximates real CPU cost much better.

Expressions
-----------

Solidity resembles the C family of languages, however it has its quirks. Simple expressions
can have the following operators: ``-``, ``+``, ``*``, ``/``, and ``%``, and the unary
operators ``-`` and ``!``::

	uint32 fahrenheit = celcius * 9 / 5 + 32;

Parentheses can be used too, of course::

	uint32 celcius = (fahrenheit - 32) * 5 / 9;

Assignment expressions are also supported, as you would expect::

	balance += 10;

It is also possible to compare values. For, this the ``>=``, ``>``, ``==``, ``!=``, ``<``, and ``<=``
is supported. This is useful for conditionals.

The post-increment and pre-increment operators are implemented like you would expect. So, ``a++``
evaluates to the value of of ``a`` before incrementing, and ``++a`` evaluates to value of ``a``
after incrementing.

The result of a comparison operator can be assigned to a bool. For example::

	bool even = (value % 2) == 0;

It is not allowed to assign an integer to a bool; an explicit comparision is needed to turn it into
a bool.

Solidity is strict about the sign of operations, and whether an assignment can truncate a value;
these are fatal errors and Solang will refuse to compile it. You can force the compiler to
accept truncations or differences in sign by adding a cast, but this is best avoided. Often
changing the parameters or return value of a function will avoid the need for casting. A code
reviewer could see cast as a code smell.

Some examples::

	function abs(int bar) public returns (int64) {
          if (bar > 0) {
                  return bar;
          } else {
                  return -bar;
      		}
  }

The compiler will say::

  implicit conversion would truncate from uint256 to uint64

Now you can work around this by adding a cast to the argument to return ``return uint64(bar);``,
however it would be much nicer if the return value matched the argument. Multiple abs() could exists
with overloaded functions, so that there is an ``abs()`` for each type.

.. note::

  The Ethereum Foundation Solidity compiler supports more expressions than are listed here.
  These will be implemented in Solang in early 2020.

Conditionals and Loops
----------------------

Contracts
---------

Enums
-----

Constructors
------------

Functions
---------
