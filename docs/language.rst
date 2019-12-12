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

Enums
-----

Solidity enums types have to be defined on the contract level. An enum has a type name, and a list of
unique values. Enum types can used in public functions, but the value is represented as a ``uint8``
in the ABI.

An enum can be converted to and from integer, but this requires an explicit cast. The value of an enum
is numbered from 0, like in C and Rust::

  contract enum_example {
      enum Weekday { Monday, Tuesday, Wednesday, Thursday, Friday, Saturday, Sunday }

      function is_weekend(Weekday day) public pure returns (bool) {
          return (day == Weekday.Saturday || day == Weekday.Sunday);
      }
  }

Solidity Source File Structure
------------------------------

A solidity file may have multiple contracts in them. If you compile a Solidity file containing::

  contract A {
      function foo() public return (bool) {
          return true;
      }
  }

  contract B {
      function bar() public return (bool) {
          return false;
      }
  }

Then Solang will output ``A.wasm`` and ``B.wasm``, along with the ABI files for each contract.

.. note::

  The Ethereum Foundation Solidity compiler can also contain other other elements other than
  contracts: ``pragma``, ``import``, ``library``, ``interface``. These are not supported yet
  and these should not be included as they may result in parser errors.

Contract Storage
----------------

Any variables declared at the contract level (so not contained in a function or constructor),
then these will automatically become contract storage. Contract storage is maintained between
calls on-chain. These are declared so::

  contract hitcount {
      uint counter = 1;

      function hit() public {
          counters++;
      }

      function count() public returns (uint) {
          return counter;
      }
  }

The ``counter`` is maintained for each deployed ``hitcount`` contract. When the contract is deployed,
the contract storage is set to 1. The ``= 1`` initializer is not required; when it is not present, it
is initialized to 0, or ``false`` if it is a ``bool``.

Constants
---------

Constants are declared at the contract level just like contract storage variables. However, they
do not use any contract storage and cannot be modified. Assigning a value to a constant is a
compiler error. The variable must have an initializer, which must be a constant expression. It is
not allowed to call functions or read variables in the initializer::

  contract ethereum {
      uint constant byzantium_block = 4_370_000;
  }

Constructors
------------

When a contract is deployed, the contract storage is initialized to the initializer values provided,
and any constructor is called. A constructor is not required for a contract. A constructor is defined
like so::

  contract mycontract {
      uint foo;

      constructor(uint foo_value) public {
          foo = foo_value;
      }
  }

A constructor does not have a name and may have any number of arguments. If a constructor has arguments,
then when the contract is deployed then those arguments must be supplied.

A constructor must be declared ``public``.

.. note::

  Parity Substrate allows multiple constructors to be defined, which is not true for Hyperledge Burrow
  or other Ethereum Style blockchains. So, when building for Substrate, multiple constructors can be
  defined as long as their argument list is different (i.e. overloaded).

  When the contract is deployed in the Polkadot UI, the user can select the constructor to be used.

.. note::

  The Ethereum Foundation Solidity compiler allows constructors to be declared ``internal`` if
  for abstract contracts. Since Solang does not support abstract contracts, this is not possible yet.

Functions
---------

Functions can be declared and called as follow::

  contact foo {
      uint bound = get_initial_bound();

      function get_initial_bound() private returns (uint) {
          return 102;
      }

      function set_bound(uint _bound) public {
          bound = _bound;
      }

      function get_with_bound(uint value) view public return (uint) {
          if (value < bound) {
              return value;
          } else {
              return bound;
          }
      }
  }

Function arguments can have any number of arguments. Function arguments may have names;
if they do not have names then they cannot be used, but they will be present in the
public interface. Return values cannot have names.

Functions which are declared ``public`` will be present in the ABI and are callable
externally. If a function is declared ``private`` then it is not callable externally,
but it can be called from within the contract.

.. note::

  The Ethereum Foundation Solidity compiler does allow return values to have names,
  and the ``return`` statement can be elided. This will be corrected in Solang
  in early 2020.

Function overloading
____________________

Multiple functions with the same name can be declared, as long as the arguments are
different in at least one of two ways:

- The number of arguments must be different
- The type of at least one of the arguments is different

A function cannot be overloaded by changing the return types or number of returned
values. Here is an example of an overloaded function::

  contract shape {
      int64 bar;

      function abs(int val) public returns (int) {
          if (val >= 0) {
              return val;
          } else {
              return -val;
          }
      }

      function abs(int64 val) public returns (int64) {
          if (val >= 0) {
              return val;
          } else {
              return -val;
          }
      }

      function foo(int64 x) public {
          bar = abs(x);
      }
  }

In the function foo, abs() is called with an ``int64`` so the second implementation
of the function abs() is called.

Function Mutability
___________________

A function which does not access any contract storage, can be declared ``pure``.
Alternatively, if a function only reads contract, but does not write to contract
storage, it can be declared ``view``.

When a function is declared either ``view`` or ``pure``, it can be called without
creating an on-chain transaction, so there is no associated gas cost.

Fallback function
_________________

When a function is called externally, either via an transaction or when one contract
call a function on another contract, the correct function is dispatched based on the
function selector in the ABI. If no function matches, then the fallback function
is called, if it is defined. If no fallback function is defined then the call aborts
via the ``unreachable`` wasm instruction. A fallback function may not have a name,
any arguments or return values, and must be declared ``external``. Here is an example of
fallback function::

  contract test {
      int32 bar;

      function foo(uint32 x) public {
          bar = x;
      }

      function() external {
          bar = 0;
      }
  }

Conditionals and Loops
----------------------



