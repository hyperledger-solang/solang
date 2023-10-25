Expressions
===========

Solidity resembles the C family of languages. Expressions can use the following operators.

Arithmetic operators
____________________

The binary operators ``-``, ``+``, ``*``, ``/``, ``%``, and ``**`` are supported, and also
in the assignment form ``-=``, ``+=``, ``*=``, ``/=``, and ``%=``. There is a
unary operator ``-``.

.. code-block:: solidity

 	uint32 fahrenheit = celsius * 9 / 5 + 32;

Parentheses can be used too, of course:

.. code-block:: solidity

 	uint32 celsius = (fahrenheit - 32) * 5 / 9;

Operators can also come in the assignment form.

.. code-block:: solidity

 	balance += 10;

The exponation (or power) can be used to multiply a number N times by itself, i.e.
x :superscript:`y`. This can only be done for unsigned types.

.. code-block:: solidity

  uint64 thousand = 1000;
  uint64 billion = thousand ** 3;

No overflow checking is generated in `unchecked` blocks, like so:

.. include:: ../examples/expression_unchecked.sol
  :code: solidity

Bitwise operators
_________________

The ``|``, ``&``, ``^`` are supported, as are the shift operators ``<<``
and ``>>``. These are also available in the assignment form ``|=``, ``&=``,
``^=``, ``<<=``, and ``>>=``. Lastly there is a unary operator ``~`` to
invert all the bits in a value.

Logical operators
_________________

The logical operators ``||``, ``&&``, and ``!`` are supported. The ``||`` and ``&&``
short-circuit. For example:

.. code-block:: javascript

  bool foo = x > 0 || bar();

bar() will not be called if the left hand expression evaluates to true, i.e. x is greater
than 0. If x is 0, then bar() will be called and the result of the ``||`` will be
the return value of bar(). Similarly, the right hand expressions of ``&&`` will not be
evaluated if the left hand expression evaluates to ``false``; in this case, whatever
ever the outcome of the right hand expression, the ``&&`` will result in ``false``.

.. code-block:: javascript

  bool foo = x > 0 && bar();

Now ``bar()`` will only be called if x *is* greater than 0. If x is 0 then the ``&&``
will result in false, irrespective of what bar() would return, so bar() is not
called at all. The expression elides execution of the right hand side, which is also
called *short-circuit*.


Conditional operator
____________________

The ternary conditional operator ``? :`` is supported:

.. code-block:: javascript

  uint64 abs = foo > 0 ? foo : -foo;


Comparison operators
____________________

It is also possible to compare values. For, this the ``>=``, ``>``, ``==``, ``!=``, ``<``, and ``<=``
is supported. This is useful for conditionals.


The result of a comparison operator can be assigned to a bool. For example:

.. code-block:: javascript

 	bool even = (value % 2) == 0;

It is not allowed to assign an integer to a bool; an explicit comparison is needed to turn it into
a bool.

Increment and Decrement operators
_________________________________

The post-increment and pre-increment operators are implemented by reading the variable's
value before or after modifying it. ``i++``returns the value of ``i`` before incrementing,
and ``++i`` returns the value of ``i`` after incrementing.

this
____

The keyword ``this`` evaluates to the current contract. The type of this is the type of the
current contract. It can be cast to ``address`` or ``address payable`` using a cast.

.. tabs::

    .. group-tab:: Polkadot

        .. include:: ../examples/polkadot/expression_this.sol
            :code: solidity


    .. group-tab:: Solana

        .. include:: ../examples/solana/expression_this.sol
            :code: solidity

Function calls made via this are function calls through the external call mechanism; i.e. they
have to serialize and deserialise the arguments and have the external call overhead. In addition,
this only works with public functions.

.. tabs::

    .. group-tab:: Polkadot

        .. include:: ../examples/polkadot/expression_this_external_call.sol
            :code: solidity


    .. group-tab:: Solana

        .. include:: ../examples/solana/expression_this_external_call.sol
            :code: solidity

.. note::

    On Solana, ``this`` returns the program account. If you are looking for the data account, please
    use ``tx.accounts.dataAccount.key``.

type(..) operators
__________________

For integer values, the minimum and maximum values the types can hold are available using the
``type(...).min`` and ``type(...).max`` operators. For unsigned integers, ``type(..).min``
will always be 0.

.. include:: ../examples/type_operator.sol
  :code: solidity

The `EIP-165 <https://eips.ethereum.org/EIPS/eip-165>`_ interface value can be retrieved using the
syntax ``type(...).interfaceId``. This is only permitted on interfaces. The interfaceId is simply
an bitwise XOR of all function selectors in the interface. This makes it possible to uniquely identify
an interface at runtime, which can be used to write a `supportsInterface()` function as described
in the EIP.

The contract code for a contract, i.e. the binary WebAssembly or Solana SBF, can be retrieved using the
``type(c).creationCode`` and ``type(c).runtimeCode`` fields, as ``bytes``. On EVM,
the constructor code is in the ``creationCode`` and all the functions are in
the ``runtimeCode``. Polkadot and Solana use the same
code for both, so those fields will evaluate to the same value.

.. include:: ../examples/retrieve_contract_code.sol
  :code: solidity

.. note::
    ``type().creationCode`` and ``type().runtimeCode`` are compile time constants.

    It is not possible to access the code for the current contract. If this were possible,
    then the contract code would need to contain itself as a constant array, which would
    result in an contract of infinite size.

Ether, Sol, and time units
__________________________

Any decimal numeric literal constant can have a unit denomination. For example
``10 minutes`` will evaluate to 600, i.e. the constant will be multiplied by the
multiplier listed below. The following units are available:

============ =========================
Unit         Multiplier

``seconds``  1
``minutes``  60
``hours``    3600
``days``     86400
``weeks``    604800
``lamports`` 1
``sol``      1_000_000_000
``wei``      1
``gwei``     1_000_000_000
``ether``    1_000_000_000_000_000_000
============ =========================

Note that the Ethereum currency denominations ``ether``, ``gwei``, and ``wei`` are available when not
compiling for Ethereum, but they will produce warnings.

Casting
_______

Solidity is very strict about the sign of operations, and whether an assignment can truncate a
value. You can force the compiler to accept truncations or sign changes by adding an
explicit cast.

Some examples:

.. code-block:: solidity

  function abs(int bar) public returns (int64) {
      if (bar > 0) {
          return bar;
      } else {
          return -bar;
      }
  }

The compiler will say:

.. code-block:: none

   implicit conversion would truncate from int256 to int64

Now you can work around this by adding a cast to the argument to return ``return int64(bar);``,
however it is idiomatic to match the return value type with the argument type. Instead, implement
multiple overloaded abs() functions, so that there is an ``abs()`` for each type.

It is allowed to cast from a ``bytes`` type to ``int`` or ``uint`` (or vice versa), only if the length
of the type is the same. This requires an explicit cast.

.. code-block:: solidity

  bytes4 selector = "ABCD";
  uint32 selector_as_uint = uint32(selector);

If the length also needs to change, then another cast is needed to adjust the length. Truncation and
extension is different for integers and bytes types. Integers pad zeros on the left when extending,
and truncate on the right. bytes pad on right when extending, and truncate on the left. For example:

.. code-block:: solidity

  bytes4 start = "ABCD";
  uint64 start1 = uint64(uint4(start));
  // first cast to int, then extend as int: start1 = 0x41424344
  uint64 start2 = uint64(bytes8(start));
  // first extend as bytes, then cast to int: start2 = 0x4142434400000000

A similar example for truncation:

.. code-block:: javascript

  uint64 start = 0xdead_cafe;
  bytes4 start1 = bytes4(uint32(start));
  // first truncate as int, then cast: start1 = hex"cafe"
  bytes4 start2 = bytes4(bytes8(start));
  // first cast, then truncate as bytes: start2 = hex"dead"

Since ``byte`` is an array of one byte, a conversion from ``byte`` to ``uint8`` requires a cast. This is
because ``byte`` is an alias for ``bytes1``.
