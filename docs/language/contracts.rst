Contracts
=========

Constructors and contract instantiation
---------------------------------------

When a contract is deployed, the contract storage is initialized to the initializer values provided,
and any constructor is called. A constructor is not required for a contract. A constructor is defined
like so:

.. code-block:: solidity

  contract mycontract {
      uint foo;

      constructor(uint foo_value) {
          foo = foo_value;
      }
  }

A constructor can have any number of arguments.
If a constructor has arguments, they must be supplied when the contract is deployed.

If a contract is expected to receive value on instantiation, the constructor should be declared ``payable``.

.. note::
    On Substrate, constructors have a name. Solang allows naming constructors in the substrate target:

    .. code-block:: solidity
 
        contract Foo {
            constructor my_new_foo() {}
        }

    Unnamed constructors will be called ``new`` in the metadata.

    Note that constructor names are only used in the generated metadata. For contract instantiation,
    the correct constructor matching the function signature will be selected automatically.

Instantiation using new
_______________________

Contracts can be created using the ``new`` keyword. The contract that is being created might have
constructor arguments, which need to be provided.

.. code-block:: solidity

    contact hatchling {
        string name;

        constructor(string id) {
            require(id != "", "name must be provided");
            name = id;
        }
    }

    contract adult {
        function test() public {
            hatchling h = new hatchling("luna");
        }
    }

The constructor might fail for various reasons, for example ``require()`` might fail here. This can
be handled using the :ref:`try-catch` statement, else errors cause the transaction to fail.

.. _sending_values:

Sending value to the new contract
_________________________________

It is possible to send value to the new contract. This can be done with the ``{value: 500}``
syntax, like so:

.. code-block:: solidity

    contact hatchling {
        string name;

        constructor(string id) payable {
            require(id != "", "name must be provided");
            name = id;
        }
    }

    contract adult {
        function test() public {
            hatchling h = new hatchling{value: 500}("luna");
        }
    }

The constructor should be declared ``payable`` for this to work.

.. note::
    If no value is specified, then on Parity Substrate the minimum balance (also know as the
    existential deposit) is sent.

Setting the salt, gas, and space for the new contract
_____________________________________________________

.. note::
    The gas or salt cannot be set on Solana. However, when creating a contract
    on Solana, the size of the new account can be set using `space:`.

When a new contract is created, the address for the new contract is a hash of the input
(the constructor arguments) to the new contract. So, a contract cannot be created twice
with the same input. This is why the salt is concatenated to the input. The salt is
either a random value or it can be explicitly set using the ``{salt: 2}`` syntax. A
constant will remove the need for the runtime random generation, however creating
a contract twice with the same salt and arguments will fail. The salt is of type
``uint256``.

If gas is specified, this limits the amount gas the constructor for the new contract
can use. gas is a ``uint64``.

.. code-block:: solidity

    contact hatchling {
        string name;

        constructor(string id) payable {
            require(id != "", "name must be provided");
            name = id;
        }
    }

    contract adult {
        function test() public {
            hatchling h = new hatchling{salt: 0, gas: 10000}("luna");
        }
    }

When creating contract on Solana, the size of the new account can be specified using
`space:`. By default, the new account is created with a size of 1 kilobyte (1024 bytes)
plus the size required for any fixed-size fields. When you specify space, this is
the space in addition to the fixed-size fields. So, if you specify `space: 0`, then there is
no space for any dynamicially allocated fields.

.. code-block:: solidity

    contact hatchling {
        string name;

        constructor(string id) payable {
            require(id != "", "name must be provided");
            name = id;
        }
    }

    contract adult {
        function test() public {
            hatchling h = new hatchling{space: 10240}("luna");
        }
    }


Base contracts, abstract contracts and interfaces
-------------------------------------------------

Solidity contracts support object-oriented programming. The style Solidity is somewhat similar to C++,
but there are many differences. In Solidity we are dealing with contracts, not classes.

Specifying base contracts
_________________________

To inherit from another contract, you have to specify it as a base contract. Multiple contracts can
be specified here.

.. code-block:: solidity

    contact a is b, c {
        constructor() {}
    }

    contact b {
        int foo;
        function func2() public {}
        constructor() {}
    }

    contact c {
        int bar;
        constructor() {}
        function func1() public {}
    }

In this case, contract ``a`` inherits from both ``b`` and ``c``. Both ``func1()`` and ``func2()``
are visible in contract ``a``, and will be part of its public interface if they are declared ``public`` or
``external``. In addition, the contract storage variables ``foo`` and ``bar`` are also availabe in ``a``.

Inheriting contracts is recursive; this means that if you inherit a contract, you also inherit everything
that that contract inherits. In this example, contract ``a`` inherits ``b`` directly, and inherits ``c``
through ``b``. This means that contract ``b`` also has a variable ``bar``.

.. code-block:: solidity

    contact a is b {
        constructor() {}
    }

    contact b is c {
        int foo;
        function func2() public {}
        constructor() {}
    }

    contact c {
        int bar;
        constructor() {}
        function func1() public {}
    }

Virtual Functions
_________________

When inheriting from a base contract, it is possible to override a function with a newer function with the same name.
For this to be possible, the base contract must have specified the function as ``virtual``. The
inheriting contract must then specify the same function with the same name, arguments and return values, and
add the ``override`` keyword.

.. code-block:: solidity

    contact a is b {
        function func(int a) override public returns (int) {
            return a + 11;
        }
    }

    contact b {
        function func(int a) virtual public returns (int) {
            return a + 10;
        }
    }

If the function is present in more than one base contract, the ``override`` attribute must list all the base
contracts it is overriding.

.. code-block:: solidity

    contact a is b,c {
        function func(int a) override(b,c) public returns (int) {
            return a + 11;
        }
    }

    contact b {
        function func(int a) virtual public returns (int) {
            return a + 10;
        }
    }

    contact c {
        function func(int a) virtual public returns (int) {
            return a + 5;
        }
    }

Calling function in base contract
_________________________________

When a virtual function is called, the dispatch is *virtual*. If the function being called is
overriden in another contract, then the overriding function is called. For example:


.. code-block:: solidity

    contract a is b {
        function baz() public returns (uint64) {
            return foo();
        }

        function foo() internal override returns (uint64) {
            return 2;
        }
    }

    contract a {
        function foo() internal virtual returns (uint64) {
            return 1;
        }

        function bar() internal returns (uint64) {
            // since foo() is virtual, is a virtual dispatch call
            // when foo is called and a is a base contract of b, then foo in contract b will
            // be called; foo will return 2.
            return foo();
        }

        function bar2() internal returns (uint64) {
            // this explicitly says "call foo of base contract a", and dispatch is not virtual
            return a.foo();
        }
    }

Rather than specifying the base contract, use ``super`` as the contract to call the base contract
function.

.. code-block:: solidity

    contract a is b {
        function baz() public returns (uint64) {
            // this will return 1
            return super.foo();
        }

        function foo() internal override returns (uint64) {
            return 2;
        }
    }

    contract b {
        function foo() internal virtual returns (uint64) {
            return 1;
        }
    }

If there are multiple base contracts which the define the same function, the function of the first base
contract is called.

.. code-block:: solidity

    contract a is b1, b2 {
        function baz() public returns (uint64) {
            // this will return 100
            return super.foo();
        }

        function foo() internal override(b2, b2) returns (uint64) {
            return 2;
        }
    }

    contract b1 {
        function foo() internal virtual returns (uint64) {
            return 100;
        }
   }

    contract b2 {
        function foo() internal virtual returns (uint64) {
            return 200;
        }
    }


Specifying constructor arguments
________________________________

If a contract inherits another contract, then when it is instantiated or deployed, then the constructor for
its inherited contracts is called. The constructor arguments can be specified on the base contract itself.

.. code-block:: solidity

    contact a is b(1) {
        constructor() {}
    }

    contact b is c(2) {
        int foo;
        function func2(int i) public {}
        constructor() {}
    }

    contact c {
        int bar;
        constructor(int32 j) {}
        function func1() public {}
    }

When ``a`` is deployed, the constructor for ``c`` is executed first, then ``b``, and lastly ``a``. When the
constructor arguments are specified on the base contract, the values must be constant. It is possible to specify
the base arguments on the constructor for inheriting contract. Now we have access to the constructor arguments,
which means we can have runtime-defined arguments to the inheriting constructors.

.. code-block:: solidity

    contact a is b {
        constructor(int i) b(i+2) {}
    }

    contact b is c {
        int foo;
        function func2() public {}
        constructor(int j) c(j+3) {}
    }

    contact c {
        int bar;
        constructor(int32 k) {}
        function func1() public {}
    }

The execution is not entirely intuitive in this case. When contract ``a`` is deployed with an int argument of 10,
then first the constructor argument or contract ``b`` is calculated: 10+2, and that value is used as an
argument to constructor ``b``. constructor ``b`` calculates the arguments for constructor ``c`` to be: 12+3. Now,
with all the arguments for all the constructors established, constructor ``c`` is executed with argument 15, then
constructor ``b`` with argument 12, and lastly constructor ``a`` with the original argument 10.

Abstract Contracts
__________________

An ``abstract contract`` is one that cannot be instantiated, but it can be used as a base for another contract,
which can be instantiated. A contract can be abstract because the functions it defines do not have a body,
for example:

.. code-block:: solidity

    abstract contact a {
        function func2() virtual public;
    }

This contract cannot be instantiated, since there is no body or implementation for ``func2``. Another contract
can define this contract as a base contract and override ``func2`` with a body.

Another reason why a contract must be abstract is missing constructor arguments. In this case, if we were to
instantiate contract ``a`` we would not know what the constructor arguments to its base ``b`` would have to be.
Note that contract ``c`` does inherit from ``a`` and can specify the arguments for ``b`` on its constructor,
even though ``c`` does not directly inherit ``b`` (but does indirectly).

.. code-block:: solidity

    abstract contact a is b {
        constructor() {}
    }

    contact b {
        constructor(int j) {}
    }

    contract c is a {
        constructor(int k) b(k*2) {}
    }
