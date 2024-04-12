``Using`` directive
===================

Binding methods to types with ``using``
---------------------------------------

Methods can be bound to builtin types and any user-defined types like structs
using the ``using`` syntax. This can be done either using libraries or free
standing functions.

``using`` with free standing functions
______________________________________

First, declare a function with one or more arguments. Once the function
is bound with ``using``, it can be called like a method.

.. include:: ../examples/using.sol
  :code: solidity

The ``using`` declaration can be done on file scope. In this case, the type must
be specified in place of ``*``. The first argument must match the type that is
be used in the ``using`` declaration.

If a user-defined type is used, the ``global`` keyword can be used. This
means the ``using`` binding can be used in any file, even when the type is
imported.

.. include:: ../examples/using_global.sol
  :code: solidity

Now even when ``User`` is imported, the clear_count() method can be used.

.. include:: ../examples/using_imports.sol
  :code: solidity

.. _user_defined_operators:

User defined Operators
______________________

The ``using`` directive can be used to bind operators for :ref:`user defined types <user_defined_types>`
to functions. A binding can be set for the operators: ``==``, ``!=``, ``>=``, ``>``, ``<=``, ``<``, ``~``,
``&``, ``|``, ``^``, ``-`` (both negate and subtract), ``+``, ``*``, ``/``, and ``%``.

First, declare a function with the correct prototype that implements the operator.

* The function must be free standing: declared outside a contract.
* The function must have ``pure`` mutability.
* All the parameters must be the same user type.
* The number of arguments depends on which operator is implemented; binary operators require two and unary operators, one.
* The function must return either ``bool`` for the comparison operators, or the same user type as the parameters for the other operators.

Then, bind the function to the operator using the syntax ``using {function-name as operator} for user-type global;``.
Operators can only be defined with ``global`` set. Note that the ``-`` operator is
used for two operators: subtract and negate. In order to bind the unary negate operator,
the function must have a single parameter. For the subtract operator, two parameters are required.

.. include:: ../examples/user_defined_operators.sol
   :code: solidity

``using`` with libraries
________________________

A library may be used for handling methods on a type. First, declare a library
with all the methods you want for a type, and then bind the library to the type
with ``using``.

.. code-block:: solidity

    struct User {
        string name;
        uint name;
    }

    library UserLibrary {
        function clear_count(User user) internal {
            user.count = 0;
        }

        function inc(User user) internal {
            user.count++;
        }

        function dec(User user) internal {
            require(user.count > 0);
            user.count--;
        }
    }

    using UserLibrary for User global;

Scope for ``using``
___________________

The ``using`` declaration may be scoped in various ways:

  - Globally by adding the ``global`` keyword. This means the methods are available
    in any file.
  - Per file, by omitting the ``global`` keyword
  - Per contract, by putting the ``using`` declaration in a contract definition

If the scope is per contract, then the type maybe be replaced with ``*`` and
the type from the first argument of the function will be used.