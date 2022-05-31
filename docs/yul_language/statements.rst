Statements
==========

For-loop
________

For-loops are completely supported in Solang. ``continue`` and ``break`` statements are also supported.
The syntax for ``for`` is quite different from other commonly known programming languages. After the ``for`` keyword,
the lexer expects a yul block delimited with curly brackets that initializes variables for the loop. This block can have
as many statements as needed and is always executed.

After the initialization block, there should be an Yul expression that contains the loop stopping condition. Then comes
the update block, which contains all the statements executed after the main body block, but before the condition check.
The body block is a set of instructions executed during each iteration.

.. code-block:: yul

    {
        function foo()
        {
            // Simple for loop
           for {let i := 0} lt(i, 10) {i := add(i, 10)} {
                let p := funcCall(i, 10)
                if eq(p, 5) {
                    continue
                }

                if eq(p, 90) {
                    break
                }
            }

            let a := 0
            // More complex loops are also possible
            for {
                let i := 0
                let j := 3
                i := add(j, i)
            } or(lt(i, 10), lt(j, 5)) {
                i := add(i, 1)
                j := add(j, 3)
            } {
                a := add(a, mul(i, j))
            }
        }
    }

If-block
________

If-block conditions in Yul cannot have an `else`. They act only as a branch if the condition is true
and are totally supported in Solang.

.. code-block:: yul

    {
        if eq(5, 4) {
            funcCall(4, 3)
        } // There cannot be an 'else' here
    }


Switch
_______

Switch statements are not yet supported in Solang. If there is urgent need to support them,
please, file a GitHub issue in the repository.

Blocks
______

There can be blocks of code within Yul, defined by curly brackets. They have their own scope and any variable
declared inside a block cannot be accessed outside it. Statements inside a block can access outside variables, though.

.. code-block:: yul

    {
        function foo() -> ret {
            let g := 0
            { // This is a code block
                let r := 7
                ret := mul(g, r)
            }
        }
    }


Variable declaration
____________________

Variables can be declared in Yul using the `let` keyword. Multiple variables can be declared at the same line
if there is no initializer or the initializer is a function that returns multiple values.

The default type for variables in Yul is ``u256``. If you want to declare a variable with another type, use the colon.
Note that if the variable type and the type of the right hand side of the assignment do not match, there will be an implicit
type conversion to the correct type.

.. code-block:: yul

    {
        let a, b, c
        let d := funCall()
        let e : u64 := funcCall()
        let g, h, i := multipleReturns()
        let j : u32, k : u8 := manyReturns()
    }

Assignments
___________

Variables can be assignment using the ``:=`` operator. If the types do not match,
the compiler performs an implicit conversion, so that the right hand side type matches that of the variable.
Multiple variables can be assigned in a single line if the right hand side is a function call that returns multiple
values.


.. code-block:: yul

    {
        a := 6
        c, d := multipleReturns()
    }

Function calls
______________

Function calls in Yul are identified by the use of parenthesis after an identifier. Standalone function
calls must not return anything. Functions that have multiple returns can only appear in an assignment or definition
of multiple variables.

.. code-block:: yul

    {
        noReturns()
        a := singleReturn()
        // multipleReturns() cannot be inside 'add'
        let g := add(a, singleReturn())
        f, d, e := multipleReturns()
    }

