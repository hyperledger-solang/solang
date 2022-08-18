Functions
=========

A function can be declared inside a contract, in which case it has access to the contracts
contract storage variables, other contract functions etc. Functions can be also be declared outside
a contract.

.. code-block:: solidity

    /// get_initial_bound is called from the constructor
    function get_initial_bound() returns (uint value) {
        value = 102;
    }

    contact foo {
        uint bound = get_initial_bound();

        /** set bound for get with bound */
        function set_bound(uint _bound) public {
            bound = _bound;
        }

        /// Clamp a value within a bound.
        /// The bound can be set with set_bound().
        function get_with_bound(uint value) view public returns (uint) {
            if (value < bound) {
                return value;
            } else {
                return bound;
            }
        }
    }

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

.. code-block:: solidity

    contract foo {
        function bar(uint32 x, bool y) public returns (uint32) {
            if (y) {
                return 2;
            }

            return 3;
        }

        function test() public {
            uint32 a = bar(102, false);
            a = bar({ y: true, x: 302 });
        }
    }

If the function has a single return value, this can be assigned to a variable. If
the function has multiple return values, these can be assigned using the :ref:`destructuring`
assignment statement:

.. code-block:: solidity

    contract foo {
        function bar1(uint32 x, bool y) public returns (address, byte32) {
            return (address(3), hex"01020304");
        }

        function bar2(uint32 x, bool y) public returns (bool) {
            return !y;
        }

        function test() public {
            (address f1, bytes32 f2) = bar1(102, false);
            bool f3 = bar2({x: 255, y: true})
        }
    }

It is also possible to call functions on other contracts, which is also known as calling
external functions. The called function must be declared public.
Calling external functions requires ABI encoding the arguments, and ABI decoding the
return values. This much more costly than an internal function call.

.. code-block:: solidity

    contract foo {
        function bar1(uint32 x, bool y) public returns (address, byte32) {
            return (address(3), hex"01020304");
        }

        function bar2(uint32 x, bool y) public returns (bool) {
            return !y;
        }
    }

    contract bar {
        function test(foo f) public {
            (address f1, bytes32 f2) = f.bar1(102, false);
            bool f3 = f.bar2({x: 255, y: true})
        }
    }

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

.. code-block:: solidity

    import {AccountMeta} from 'solana';

    contract SplToken {
        address constant tokenProgramId = address"TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
        address constant SYSVAR_RENT_PUBKEY = address"SysvarRent111111111111111111111111111111111";

        struct InitializeMintInstruction {
            uint8 instruction;
            uint8 decimals;
            address mintAuthority;
            uint8 freezeAuthorityOption;
            address freezeAuthority;
        }

        function create_mint_with_freezeauthority(uint8 decimals, address mintAuthority, address freezeAuthority) public {
            InitializeMintInstruction instr = InitializeMintInstruction({
                instruction: 0,
                decimals: decimals,
                mintAuthority: mintAuthority,
                freezeAuthorityOption: 1,
                freezeAuthority: freezeAuthority
            });

            AccountMeta[2] metas = [
                AccountMeta({pubkey: instr.mintAuthority, is_writable: true, is_signer: false}),
                AccountMeta({pubkey: SYSVAR_RENT_PUBKEY, is_writable: false, is_signer: false})
            ];

            tokenProgramId.call{accounts: metas}(instr);
        }
    }

If ``{accounts}`` is not specified, then all account are passed.

Passing seeds with external calls on Solana
___________________________________________

The Solana runtime allows you to specify the seeds to be passed for an
external call. This is used for program derived addresses: the seeds are
hashed with the calling program id to create program derived addresses.
They will automatically have the signer bit set, which allows a contract to
sign without using any private keys.

.. code-block:: solidity

    import 'solana';

    contract c {
        address constant program_id = address"mv3ekLzLbnVPNxjSKvqBpU3ZeZXPQdEC3bp5MDEBG68";

        function test(address addr, address addr2, bytes seed) public {
            bytes instr = new bytes(1);

            instr[0] = 1;

            AccountMeta[2] metas = [
                AccountMeta({pubkey: addr, is_writable: true, is_signer: true}),
                AccountMeta({pubkey: addr2, is_writable: true, is_signer: true})
            ];

            token.call{accounts: metas, seeds: [ [ "test", seed ], [ "foo", "bar "] ]}(instr);
        }
    }

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

.. code-block:: solidity

    contract foo {
        function bar() public {
            other o = new other();

            o.feh{value: 102, gas: 5000}(102);
        }
    }

    contract other {
        function feh(uint32 x) public payable {
            // ...
        }
    }

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
what the runtime program uses to determine what function was called. Usually the
function selector is generated using a deterministic hash value of the function
name and the arguments types.

The selector value can be overriden with the ``selector=hex"deadbea1"`` syntax,
for example:

.. code-block:: solidity

    contract foo {
        function get_foo() selector=hex"01020304" public returns (int) {
            return 102;
        }
    }

Only ``public`` and ``external`` functions have a selector, and can have their
selector overriden. On Substrate, constructors have selectors too, so they
can also have their selector overriden. If a function overrides another one in a
base contract, then the selector of both must match.

.. warning::
    On ewasm and Solana, changing the selector may result in a mismatch between
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

.. code-block:: solidity

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

Function Modifiers
__________________

Function modifiers are used to check pre-conditions or post-conditions for a function call. First a
new modifier must be declared which looks much like a function, but uses the ``modifier``
keyword rather than ``function``.

.. code-block:: solidity

    contract example {
        address owner;

        modifier only_owner() {
            require(msg.sender == owner);
            _;
            // insert post conditions here
        }

        function foo() only_owner public {
            // ...
        }
    }

The function `foo` can only be run by the owner of the contract, else the ``require()`` in its
modifier will fail. The special symbol ``_;`` will be replaced by body of the function. In fact,
if you specify ``_;`` twice, the function will execute twice, which might not be a good idea.

A modifier cannot have visibility (e.g. ``public``) or mutability (e.g. ``view``) specified,
since a modifier is never externally callable. Modifiers can only be used by attaching them
to functions.

A modifier can have arguments, just like regular functions. Here if the price is less
than 50, `foo()` itself will never be executed, and execution will return to the caller with
nothing done since ``_;`` is not reached in the modifier and as result foo() is never
executed.

.. code-block:: solidity

    contract example {
        modifier check_price(int64 price) {
            if (price >= 50) {
                _;
            }
        }

        function foo(int64 price) check_price(price) public {
            // ...
        }
    }

Multiple modifiers can be applied to single function. The modifiers are executed in the
order of the modifiers specified on the function declaration. Execution will continue to the next modifier
when the ``_;`` is reached. In
this example, the `only_owner` modifier is run first, and if that reaches ``_;``, then
`check_price` is executed. The body of function `foo()` is only reached once `check_price()`
reaches ``_;``.

.. code-block:: solidity

    contract example {
        address owner;

        // a modifier with no arguments does not need "()" in its declaration
        modifier only_owner {
            require(msg.sender == owner);
            _;
        }

        modifier check_price(int64 price) {
            if (price >= 50) {
                _;
            }
        }

        function foo(int64 price) only_owner check_price(price) public {
            // ...
        }
    }

Modifiers can be inherited or declared ``virtual`` in a base contract and then overriden, exactly like
functions can be.

.. code-block:: solidity

    contract base {
        address owner;

        modifier only_owner {
            require(msg.sender == owner);
            _;
        }

        modifier check_price(int64 price) virtual {
            if (price >= 10) {
                _;
            }
        }
    }

    contract example is base {
        modifier check_price(int64 price) override {
            if (price >= 50) {
                _;
            }
        }

        function foo(int64 price) only_owner check_price(price) public {
            // ...
        }
    }


Calling an external function using ``call()``
_____________________________________________

If you call a function on a contract, then the function selector and any arguments
are ABI encoded for you, and any return values are decoded. Sometimes it is useful
to call a function without abi encoding the arguments.

You can call a contract directly by using the ``call()`` method on the address type.
This takes a single argument, which should be the ABI encoded arguments. The return
values are a ``boolean`` which indicates success if true, and the ABI encoded
return value in ``bytes``.

.. code-block:: solidity

    contract a {
        function test() public {
            b v = new b();

            // the following four lines are equivalent to "uint32 res = v.foo(3,5);"

            // Note that the signature is only hashed and not parsed. So, ensure that the
            // arguments are of the correct type.
            bytes data = abi.encodeWithSignature("foo(uint32,uint32)", uint32(3), uint32(5));

            (bool success, bytes rawresult) = address(v).call(data);

            assert(success == true);

            uint32 res = abi.decode(rawresult, (uint32));

            assert(res == 8);
        }
    }

    contract b {
        function foo(uint32 a, uint32 b) public returns (uint32) {
            return a + b;
        }
    }

Any value or gas limit can be specified for the external call. Note that no check is done to see
if the called function is ``payable``, since the compiler does not know what function you are
calling.

.. code-block:: solidity

    function test(address foo, bytes rawcalldata) public {
        (bool success, bytes rawresult) = foo.call{value: 102, gas: 1000}(rawcalldata);
    }

.. note::

    ewasm also supports ``staticcall()`` and ``delegatecall()`` on the address type. These
    call types are not supported on Parity Substrate.

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

.. code-block:: solidity

    contract test {
        int32 bar;

        function foo(uint32 x) public {
            bar = x;
        }

        fallback() external {
            // execute if function selector does not match "foo(uint32)" and no value sent
        }

        receive() payable external {
            // execute if function selector does not match "foo(uint32)" and value sent
        }
    }

..  note::
    On Solana, there is no mechanism to have some code executed if an account
    gets credited. So, `receive()` functions are not supported.