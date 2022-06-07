Functions
=========

Functions in Yul cannot access any variable outside their scope, i.e. they can
only operate with the variables they receive as arguments. You can define types for arguments and returns. If no
type is specified, the compiler will default to ``u256``.

.. warning::
    Yul functions are only available within the scope there are defined. They cannot be accessed from Solidity
    and from other inline assembly blocks, even if they are contained within the same Solidity function.

Differently from solc, Solang allows name shadowing inside Yul
functions. As they cannot access variables declared outside them, a redefinition of an outside name is allowed, if
it has not been declared within the function yet. Builtin function names cannot be overdriven and verbatim functions
supported by Solc are not implemented in Solang. ``verbatim``, nevertheless, is still a reserved keyword and
cannot be the prefix of variable or function names.

Function calls are identified by a name followed by parenthesis. If the types of the arguments passed to function
calls do not match the respective parameter's type, Solang will implicitly convert them. Likewise, the returned
values of function calls will be implicitly converted to match the type needed in an expression context.


.. code-block:: yul

    {
        // return type defaulted to u256
        function noArgs() -> ret
        {
            ret := 2
        }

        // Parameters defaulted to u256 and ret has type u64
        function sum(a, b) -> ret : u64
        {
            ret := add(b, a)
        }

        function getMod(c : s32, d : u128, e) -> ret1, ret2 : u64
        {
            ret1 := mulmod(c, d, e)
            ret2 := addmod(c, d, e)
        }

        function noReturns(a, b)
        {
            // Syntax of function calls
            let x := noArgs()
            // Arguments will be implicitly converted form u256 to s32 and u128, respectively.
            // The returns will also be converted to u256
            let c, d := getMod(a, b)
            {
                // 'doThis' cannot be called from outside the block defined the by curly brackets.
                function doThis(f, g) -> ret {
                    ret := sdiv(f, g)
                }
            }
            // 'doThat' cannot be called from outside 'noReturns'
            function doThat(f, g) -> ret {
                ret := smod(g, f)
            }
        }
    }