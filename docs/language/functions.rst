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

Functions which are declared ``public`` will be present in the ABI and are callable
externally. If a function is declared ``private`` then it is not callable externally,
but it can be called from within the contract. If a function is defined outside a
contract, then it cannot have a visibility specifier (e.g. ``public``).

Any DocComment before a function will be include in the ABI. Currently only Substrate
supports documentation in the ABI.

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

.. include:: ../examples/function_call_external.sol
  :code: solidity

The syntax for calling external call is the same as the external call, except for
that it must be done on a contract type variable. Any error in an external call can
be handled with :ref:`try-catch`.

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

Passing accounts with external calls on Solana
______________________________________________

The Solana runtime allows you the specify the accounts to be passed for an
external call. This is specified in an array of the struct ``AccountMeta``,
see the section on :ref:`account_meta`.

.. include:: ../examples/solana/function_call_external_accounts.sol
  :code: solidity

If ``{accounts}`` is not specified, then all account are passed.

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

.. include:: ../examples/substrate/function_call_external_gas.sol
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
    If value is sent to a non-payable function on Parity Substrate, the call will be
    reverted. However there is no refund performed, so value will remain with the callee.

    ``payable`` on constructors is not enforced on Parity Substrate. Funds are needed
    for storage rent and there is a minimum deposit needed for the contract. As a result,
    constructors always receive value on Parity Substrate.

Overriding function selector
____________________________

When a function is called, the function selector and the arguments are serialized
(also known as abi encoded) and passed to the program. The function selector is
what the runtime program uses to determine what function was called. On Substrate, the
function selector is generated using a deterministic hash value of the function
name and the arguments types. On Solana, the selector is known as discriminator.

The selector value can be overriden with the annotation
``@selector([0xde, 0xad, 0xbe, 0xa1])``.

.. include:: ../examples/substrate/function_selector_override.sol
  :code: solidity

The given example only works for Substrate, whose selectors are four bytes wide. On Solana, they are eight bytes wide.

Only ``public`` and ``external`` functions have a selector, and can have their
selector overriden. On Substrate, constructors have selectors too, so they
can also have their selector overriden. If a function overrides another one in a
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

Both Substrate and Solana runtime require unique function names, so
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

.. include:: ../examples/substrate/function_modifier.sol
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

.. include:: ../examples/substrate/function_multiple_modifiers.sol
  :code: solidity

Modifiers can be inherited or declared ``virtual`` in a base contract and then overriden, exactly like
functions can be.

.. include:: ../examples/substrate/function_override_modifiers.sol
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

.. include:: ../examples/function_call.sol
  :code: solidity

Any value or gas limit can be specified for the external call. Note that no check is done to see
if the called function is ``payable``, since the compiler does not know what function you are
calling.

.. code-block:: solidity

    function test(address foo, bytes rawcalldata) public {
        (bool success, bytes rawresult) = foo.call{value: 102, gas: 1000}(rawcalldata);
    }

.. _fallback_receive:

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

.. include:: ../examples/substrate/function_fallback_and_receive.sol
  :code: solidity

..  note::
    On Solana, there is no mechanism to have some code executed if an account
    gets credited. So, `receive()` functions are not supported.
