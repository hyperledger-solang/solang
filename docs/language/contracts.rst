Contracts
=========

Constructors and contract instantiation
---------------------------------------

When a contract is deployed, the contract storage is initialized to the initializer values provided,
and any constructor is called. A constructor is not required for a contract. A constructor is defined
like so:

.. include:: ../../examples/substrate/flipper.sol
  :code: solidity

A constructor can have any number of arguments.
If a constructor has arguments, they must be supplied when the contract is deployed.

If a contract is expected to receive value on instantiation, the constructor should be declared ``payable``.

.. note::
    Solang allows naming constructors in the Substrate target:

    .. include:: ../examples/substrate/constructor_named.sol
      :code: solidity

    Constructors without a name will be called ``new`` in the metadata.

    Note that constructor names are only used in the generated metadata. For contract instantiation,
    the correct constructor matching the function signature will be selected automatically.

Instantiation using new
_______________________

Contracts can be created using the ``new`` keyword. The contract that is being created might have
constructor arguments, which need to be provided.

.. include:: ../examples/substrate/contract_new.sol
  :code: solidity

On Solana, the contract being created must have the ``@program_id()`` annotation, and the address
of the new account must be specified using the ``{address: new_address}`` syntax.

.. include:: ../examples/substrate/contract_new.sol
  :code: solidity

The constructor might fail for various reasons, for example ``require()`` might fail here. This can
be handled using the :ref:`try-catch` statement, else errors cause the transaction to fail.

.. note::
  On Solana, the :ref:`try-catch` statement is not supported, as any failure will
  cause the entire transaction to fail.

.. _sending_values:

Sending value to the new contract
_________________________________

It is possible to send value to the new contract. This can be done with the ``{value: 500}``
syntax, like so:

.. include:: ../examples/substrate/contract_payable.sol
  :code: solidity

The constructor should be declared ``payable`` for this to work.

.. note::
    If no value is specified, then on Parity Substrate the minimum balance (also know as the
    existential deposit) is sent.

.. note::
    On Solana, this functionality is not available.

Setting the salt, gas, and address for the new contract
_______________________________________________________

.. note::
    The gas or salt cannot be set on Solana. However, when creating a contract
    on Solana, the address of the new account must be set using ``address:``.

When a new contract is created, the address for the new contract is a hash of the input
(the constructor arguments) to the new contract. So, a contract cannot be created twice
with the same input. This is why the salt is concatenated to the input. The salt is
either a random value or it can be explicitly set using the ``{salt: 2}`` syntax. A
constant will remove the need for the runtime random generation, however creating
a contract twice with the same salt and arguments will fail. The salt is of type
``uint256``.

If gas is specified, this limits the amount gas the constructor for the new contract
can use. gas is a ``uint64``.

.. include:: ../examples/substrate/contract_gas_limit.sol
  :code: solidity

.. _solana_constructor:

Instantiating a contract on Solana
__________________________________

On Solana, contracts are deployed to a program account, which holds only the contract's executable binary.
When calling a constructor, one needs to provide an address that will serve as the contract's data account,
by using the call argument ``address``:

.. include:: ../examples/solana/contract_address.sol
  :code: solidity

Alternatively, the data account to be initialized can be provided using the ``accounts`` call argument. In this case,
one needs to instantiate a fixed length array of type ``AccountMeta`` to pass to the call. The array must contain all
the accounts the transaction is going to need, in addition to the data account to be initialized.

For the creation of a contract, the data account must the **first** element in such a vector and the system account
``11111111111111111111111111111111`` must also be present. If the constructor one is calling has the
:ref:`@payer annotation <payer_seeds_bump>`, the payer account should appear in the array as well. Moreover, the
``is_signer`` and ``is_writable`` bool flags need to be properly set, according to the following example:


.. include:: ../examples/solana/create_contract_with_metas.sol
  :code: solidity


Base contracts, abstract contracts and interfaces
-------------------------------------------------

Solidity contracts support object-oriented programming. The style Solidity is somewhat similar to C++,
but there are many differences. In Solidity we are dealing with contracts, not classes.

Specifying base contracts
_________________________

To inherit from another contract, you have to specify it as a base contract. Multiple contracts can
be specified here.

.. include:: ../examples/contract_inheritance.sol
  :code: solidity

In this case, contract ``a`` inherits from both ``b`` and ``c``. Both ``func1()`` and ``func2()``
are visible in contract ``a``, and will be part of its public interface if they are declared ``public`` or
``external``. In addition, the contract storage variables ``foo`` and ``bar`` are also availabe in ``a``.

Inheriting contracts is recursive; this means that if you inherit a contract, you also inherit everything
that that contract inherits. In this example, contract ``a`` inherits ``b`` directly, and inherits ``c``
through ``b``. This means that contract ``b`` also has a variable ``bar``.

.. include:: ../examples/contract_recursive_inheritance.sol
  :code: solidity

Virtual Functions
_________________

When inheriting from a base contract, it is possible to override a function with a newer function with the same name.
For this to be possible, the base contract must have specified the function as ``virtual``. The
inheriting contract must then specify the same function with the same name, arguments and return values, and
add the ``override`` keyword.

.. include:: ../examples/virtual_functions.sol
  :code: solidity

If the function is present in more than one base contract, the ``override`` attribute must list all the base
contracts it is overriding.

.. include:: ../examples/virtual_functions_override.sol
  :code: solidity

Calling function in base contract
_________________________________

When a virtual function is called, the dispatch is *virtual*. If the function being called is
overriden in another contract, then the overriding function is called. For example:

.. include:: ../examples/base_contract_function_call.sol
  :code: solidity

Rather than specifying the base contract, use ``super`` as the contract to call the base contract
function.

.. include:: ../examples/super_contract_function_call.sol
  :code: solidity

If there are multiple base contracts which the define the same function, the function of the first base
contract is called.

.. include:: ../examples/contract_multiple_inheritance.sol
  :code: solidity

Specifying constructor arguments
________________________________

If a contract inherits another contract, then when it is instantiated or deployed, then the constructor for
its inherited contracts is called. The constructor arguments can be specified on the base contract itself.

.. include:: ../examples/inherited_constructor_arguments.sol
  :code: solidity

When ``a`` is deployed, the constructor for ``c`` is executed first, then ``b``, and lastly ``a``. When the
constructor arguments are specified on the base contract, the values must be constant. It is possible to specify
the base arguments on the constructor for inheriting contract. Now we have access to the constructor arguments,
which means we can have runtime-defined arguments to the inheriting constructors.

.. include:: ../examples/inherited_constructor_runtime_arguments.sol
  :code: solidity

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


.. include:: ../examples/abstract_contract.sol
  :code: solidity

This contract cannot be instantiated, since there is no body or implementation for ``func2``. Another contract
can define this contract as a base contract and override ``func2`` with a body.

Another reason why a contract must be abstract is missing constructor arguments. In this case, if we were to
instantiate contract ``a`` we would not know what the constructor arguments to its base ``b`` would have to be.
Note that contract ``c`` does inherit from ``a`` and can specify the arguments for ``b`` on its constructor,
even though ``c`` does not directly inherit ``b`` (but does indirectly).

.. include:: ../examples/abstract_contract_inheritance.sol
  :code: solidity
