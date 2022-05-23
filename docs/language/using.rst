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

.. code-block:: javascript

    function mask(uint v, uint bits) returns (uint) {
        return v & ((1 << bits) - 1);
    }

    function odd(uint v) returns (bool) {
        return (v & 1) != 0;
    }

    contract c {
        using {mask, odd} for *;

        int v;

        function set_v(int n) public {
            v = n.mask(16);
        }
    }

The ``using`` declaration can be done on file scope. In this case, the type must
be specified in place of ``*``. The first argument must match the type that is
be used in the ``using`` declaration.

If a user-defined type is used, the the ``global`` keyword can be used. This
means the ``using`` binding can be used in any file, even when the type is
imported.

.. code-block:: solidity

    struct User {
        string name;
        uint count;
    }

    function clear_count(User memory user) {
        user.count = 0;
    }

    using {clear_count} for User global;

Now even when ``User`` is imported, the clear_count() method can be used.


.. code-block:: solidity

    import {User} from "user.sol";

    contract c {
        function foo(User memory user) {
            user.clear_count();
        }
    }

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