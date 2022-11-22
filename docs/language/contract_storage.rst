Contract Storage
================

Any variables declared at the contract level (so not declared in a function or constructor),
will automatically become contract storage. Contract storage is maintained on chain, so they
retain their values between calls. These are declared so:

.. include:: ../examples/contract_storage.sol
  :code: solidity

The ``counter`` is maintained for each deployed ``hitcount`` contract. When the contract is deployed,
the contract storage is set to 1. Contract storage variable do not need an initializer; when
it is not present, it is initialized to 0, or ``false`` if it is a ``bool``.

Immutable Variables
___________________

A variable can be declared `immutable`. This means that it may only be modified in a constructor,
and not in any other function or modifier.

.. include:: ../examples/contract_storage_immutable.sol
  :code: solidity

This is purely a compiler syntax feature, the generated code is exactly the same.

Accessor Functions
__________________

Any contract storage variable which is declared public, automatically gets an accessor function. This
function has the same name as the variable name. So, in the example above, the value of counter can
retrieved by calling a function called ``counter``, which returns ``uint``.

If the type is either an array or a mapping, the key or array indices become arguments to the accessor
function.

.. include:: ../examples/contract_storage_accessor.sol
  :code: solidity

The accessor function may override a method on a base contract by specifying ``override``. The base function
must be virtual and have the same signature as the accessor. The ``override`` keyword only affects the
accessor function, so it can only be used in combination with public variables and cannot be used to
override a variable in the base contract.

.. include:: ../examples/contract_storage_accessor_override.sol
  :code: solidity

How to clear Contract Storage
_____________________________

Any contract storage variable can have its underlying contract storage cleared with the ``delete``
operator. This can be done on any type; a simple integer, an array element, or the entire
array itself. Contract storage has to be cleared slot (i.e. primitive) at a time, so if there are
many primitives, this can be costly.

.. include:: ../examples/contract_storage_clear.sol
  :code: solidity
