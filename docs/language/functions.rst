Functions
=========

A function can be declared inside a contract, in which case it has access to the contracts
contract storage variables, other contract functions etc. Functions can be also be declared outside
a contract.

.. include:: ../examples/functions.sol
  :code: solidity

Function can have any number of arguments. Function arguments may have names;
if they do not have names then they cannot be used in the function body, but they will
be present in the public interface.

The return values may have names as demonstrated in the get_initial_bound() function.
When at all of the return values have a name, then the return statement is no
longer required at the end of a function body. In stead of returning the values
which are provided in the return statement, the values of the return variables at the end
of the function is returned. It is still possible to explicitly return some values
with a return statement.

Any DocComment before a function will be include in the ABI. Currently only Polkadot
supports documentation in the ABI.

Function visibility
___________________

Solidity functions have a visibility specifier that restricts the scope in which they can be called.
Functions can be declared public, private, internal or external with the following definitions:

    - ``public`` functions can be called inside and outside a contract (e.g. by an RPC). They are
      present in the contract's ABI or IDL.
    - ``private`` functions can only be called inside the contract they are declared.
    - ``internal`` functions can only be called internally within the contract or by any contract
      inherited contract.
    - ``external`` functions can exclusively be called by other contracts or directly by an RPC. They
      are also present in the contract's ABI or IDL.

Both public and external functions can be called using the syntax ``this.func()``. In this case, the
arguments are ABI encoded for the call, as it is treated like an external call. This is the only way to
call an external function from inside the same contract it is defined. This method, however, should be avoided
for public functions, as it will be more costly to call them than simply using ``func()``.

If a function is defined outside a contract, it cannot have a visibility specifier (e.g. ``public``).


Arguments passing and return values
___________________________________

Function arguments can be passed either by position or by name. When they are called
by name, arguments can be in any order. However, functions with anonymous arguments
(arguments without name) cannot be called this way.

.. include:: ../examples/function_arguments.sol
  :code: solidity

If the function has a single return value, this can be assigned to a variable. If
the function has multiple return values, these can be assigned using the :ref:`destructuring`
assignment statement:

.. include:: ../examples/function_destructing_arguments.sol
  :code: solidity

It is also possible to call functions on other contracts, which is also known as calling
external functions. The called function must be declared public.
Calling external functions requires ABI encoding the arguments, and ABI decoding the
return values. This much more costly than an internal function call.


.. tabs::

    .. group-tab:: Polkadot

        .. include:: ../examples/polkadot/function_call_external.sol
            :code: solidity


    .. group-tab:: Solana

        .. include:: ../examples/solana/function_call_external.sol
            :code: solidity



The syntax for calling a contract is the same as that of the external call, except
that it must be done on a contract type variable. Errors in external calls can
be handled with :ref:`try-catch` only on Polkadot.

Internal calls and externals calls
___________________________________

An internal function call is executed by the current contract. This
is much more efficient than an external call, which requires the
address of the contract to call, whose arguments must be *abi encoded* (also known
as serialization). Then, the runtime must set up the VM for the called contract
(the callee), decode the arguments, and encode return values. Lastly,
the first contract (the caller) must decode return values.

A method call done on a contract type will always be an external call.
Note that ``this`` returns the current contract, so ``this.foo()`` will do an
external call, which is much more expensive than ``foo()``.

.. _solana_external_call:

Passing accounts with external calls on Solana
______________________________________________

The Solana runtime allows you the specify the accounts to be passed for an
external call. This is specified in an array of the struct ``AccountMeta``,
see the section on :ref:`account_meta`.

.. include:: ../examples/solana/function_call_external_accounts.sol
  :code: solidity

If ``{accounts}`` is not specified, all accounts passed to the current transaction are forwarded to the call.

Passing seeds with external calls on Solana
___________________________________________

The Solana runtime allows you to specify the seeds to be passed for an
external call. This is used for program derived addresses: the seeds are
hashed with the calling program id to create program derived addresses.
They will automatically have the signer bit set, which allows a contract to
sign without using any private keys.

.. include:: ../examples/solana/function_call_external_seeds.sol
  :code: solidity

Now if the program derived address for the running program id and the seeds match the address
``addr`` and ``addr2``, then then the called program will run with signer and writable bits
set for ``addr`` and ``addr2``. If they do not match, the Solana runtime will detect that
the ``is_signer`` is set without the correct signature being provided.

The seeds can provided in any other, which will be used to sign for multiple accounts. In the example
above, the seed ``"test"`` is concatenated with the value of ``seed``, and that produces
one account signature. In adition, ``"foo"`` is concatenated with ``"bar"`` to produce ``"foobar"``
and then used to sign for another account.

The ``seeds:`` call parameter is a slice of bytes slices; this means the literal can contain any
number of elements, including 0 elements. The values can be ``bytes`` or anything that can be
cast to ``bytes``.

.. _passing_value_gas:

Passing value and gas with external calls
_________________________________________

For external calls, value can be sent along with the call. The callee must be
``payable``. Likewise, a gas limit can be set.

.. include:: ../examples/polkadot/function_call_external_gas.sol
  :code: solidity

.. note::
    The gas cannot be set on Solana for external calls.


State mutability
________________

Some functions only read contract storage (also known as *state*), and others may write
contract storage. Functions that do not write state can be executed off-chain. Off-chain
execution is faster, does not require write access, and does not need any balance.

Functions that do not write state come in two flavours: ``view`` and ``pure``. ``pure``
functions may not read state, and ``view`` functions that do read state.

Functions that do write state come in two flavours: ``payable`` and non-payable, the
default. Functions that are not intended to receive any value, should not be marked
``payable``. The compiler will check that every call does not included any value, and
there are runtime checks as well, which cause the function to be reverted if value is
sent.

A constructor can be marked ``payable``, in which case value can be passed with the
constructor.

.. note::
    If value is sent to a non-payable function on Polkadot, the call will be reverted.


Overriding function selector
____________________________

When a function is called, the function selector and the arguments are serialized
(also known as abi encoded) and passed to the program. The function selector is
what the runtime program uses to determine what function was called. On Polkadot, the
function selector is generated using a deterministic hash value of the function
name and the arguments types. On Solana, the selector is known as discriminator.

The selector value can be overridden with the annotation
``@selector([0xde, 0xad, 0xbe, 0xa1])``.

.. include:: ../examples/polkadot/function_selector_override.sol
  :code: solidity

The given example only works for Polkadot, whose selectors are four bytes wide. On Solana, they are eight bytes wide.

Only ``public`` and ``external`` functions have a selector, and can have their
selector overridden. On Polkadot, constructors have selectors too, so they
can also have their selector overridden. If a function overrides another one in a
base contract, then the selector of both must match.

.. warning::
    On Solana, changing the selector may result in a mismatch between
    the contract metadata and the actual contract code, because the metadata does
    not explicitly store the selector.

    Use this feature carefully, as it may either break a contract or cause
    undefined behavior.

Function overloading
____________________

Multiple functions with the same name can be declared, as long as the arguments are
different in at least one of two ways:

- The number of arguments must be different
- The type of at least one of the arguments is different

A function cannot be overloaded by changing the return types or number of returned
values. Here is an example of an overloaded function:

.. include:: ../examples/function_overloading.sol
  :code: solidity

In the function foo, abs() is called with an ``int64`` so the second implementation
of the function abs() is called.

Both Polkadot and Solana runtime require unique function names, so
overloaded function names will be mangled in the ABI or the IDL.
The function name will be concatenated with all of its argument types, separated by underscores, using the
following rules:

- Struct types are represented by their field types (preceded by an extra underscore).
- Enum types are represented as their underlying ``uint8`` type.
- Array types are recognizable by having ``Array`` appended.
- Fixed size arrays will additionally have their length appended as well.

The following example illustrates some overloaded functions and their mangled name:

.. include:: ../examples/function_name_mangling.sol
  :code: solidity


Function Modifiers
__________________

Function modifiers are used to check pre-conditions or post-conditions for a function call. First a
new modifier must be declared which looks much like a function, but uses the ``modifier``
keyword rather than ``function``.

.. include:: ../examples/polkadot/function_modifier.sol
  :code: solidity

The function `foo` can only be run by the owner of the contract, else the ``require()`` in its
modifier will fail. The special symbol ``_;`` will be replaced by body of the function. In fact,
if you specify ``_;`` twice, the function will execute twice, which might not be a good idea.

On Solana, ``msg.sender`` does not exist, so the usual way to implement a similar test is using
an `authority` accounts rather than an owner account.

.. include:: ../examples/solana/use_authority.sol
  :code: solidity

A modifier cannot have visibility (e.g. ``public``) or mutability (e.g. ``view``) specified,
since a modifier is never externally callable. Modifiers can only be used by attaching them
to functions.

A modifier can have arguments, just like regular functions. Here if the price is less
than 50, `foo()` itself will never be executed, and execution will return to the caller with
nothing done since ``_;`` is not reached in the modifier and as result foo() is never
executed.

.. include:: ../examples/function_modifier_arguments.sol
  :code: solidity

Multiple modifiers can be applied to single function. The modifiers are executed in the
order of the modifiers specified on the function declaration. Execution will continue to the next modifier
when the ``_;`` is reached. In
this example, the `only_owner` modifier is run first, and if that reaches ``_;``, then
`check_price` is executed. The body of function `foo()` is only reached once `check_price()`
reaches ``_;``.

.. include:: ../examples/polkadot/function_multiple_modifiers.sol
  :code: solidity

Modifiers can be inherited or declared ``virtual`` in a base contract and then overridden, exactly like
functions can be.

.. include:: ../examples/polkadot/function_override_modifiers.sol
  :code: solidity

Calling an external function using ``call()``
_____________________________________________

If you call a function on a contract, then the function selector and any arguments
are ABI encoded for you, and any return values are decoded. Sometimes it is useful
to call a function without abi encoding the arguments.

You can call a contract directly by using the ``call()`` method on the address type.
This takes a single argument, which should be the ABI encoded arguments. The return
values are a ``boolean`` which indicates success if true, and the ABI encoded
return value in ``bytes``.

.. tabs::

    .. group-tab:: Polkadot

        .. include:: ../examples/polkadot/function_call.sol
            :code: solidity


    .. group-tab:: Solana

        .. include:: ../examples/solana/function_call.sol
            :code: solidity

Any value or gas limit can be specified for the external call. Note that no check is done to see
if the called function is ``payable``, since the compiler does not know what function you are
calling.

.. code-block:: solidity

    function test(address foo, bytes rawcalldata) public {
        (bool success, bytes rawresult) = foo.call{value: 102, gas: 1000}(rawcalldata);
    }

External calls with the ``call()`` method on Solana must have the ``accounts`` call argument, regardless of the
callee function visibility, because the compiler has no information about the caller function to generate the
``AccountMeta`` array automatically.

.. code-block:: solidity

    function test(address foo, bytes rawcalldata) public {
        (bool success, bytes rawresult) = foo.call{accounts: []}(rawcalldata);
    }

.. _fallback_receive:

Calling an external function using ``delegatecall``
___________________________________________________

External functions can also be called using ``delegatecall``.
The difference to a regular ``call`` is that  ``delegatecall`` executes the callee code in the context of the caller:

* The callee will read from and write to the `caller` storage.
* ``value`` can't be specified for ``delegatecall``; instead it will always stay the same in the callee.
* ``msg.sender`` does not change; it stays the same as in the callee.

Refer to the `contracts pallet <https://docs.rs/pallet-contracts/latest/pallet_contracts/api_doc/trait.Version0.html#tymethod.delegate_call>`_ 
and `Ethereum Solidity <https://docs.soliditylang.org/en/latest/introduction-to-smart-contracts.html#delegatecall-and-libraries>`_
documentations for more information.

``delegatecall`` is commonly used to implement re-usable libraries and 
`upgradeable contracts <https://docs.openzeppelin.com/upgrades-plugins/1.x/writing-upgradeable>`_.

.. code-block:: solidity

    function delegate(
    	address callee,
    	bytes input
    ) public returns(bytes result) {
        (bool ok, result) = callee.delegatecall(input);
        require(ok);
    }

..  note::
    ``delegatecall`` is not available on Solana.

..  note::
    On Polkadot, specifying ``gas`` won't have any effect on ``delegatecall``.

fallback() and receive() function
_________________________________

When a function is called externally, either via an transaction or when one contract
call a function on another contract, the correct function is dispatched based on the
function selector in the raw encoded ABI call data. If there is no match, the call
reverts, unless there is a ``fallback()`` or ``receive()`` function defined.

If the call comes with value, then ``receive()`` is executed, otherwise ``fallback()``
is executed. This made clear in the declarations; ``receive()`` must be declared
``payable``, and ``fallback()`` must not be declared ``payable``. If a call is made
with value and no ``receive()`` function is defined, then the call reverts, likewise if
call is made without value and no ``fallback()`` is defined, then the call also reverts.

Both functions must be declared ``external``.

.. include:: ../examples/polkadot/function_fallback_and_receive.sol
  :code: solidity

..  note::
    On Solana, there is no mechanism to have some code executed if an account
    gets credited. So, `receive()` functions are not supported.
