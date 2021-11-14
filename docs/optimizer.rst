Optimizer Passes
================

Solang generates its own internal IR, before the LLVM IR is generated. This internal IR allows us to do
several optimizations which LLVM cannot do, since it is not aware of higher-level language constructs.

Arithmetic of large integers (larger than 64 bit) has special handling, since LLVM cannot generate them.
So we need to do our own optimizations for these types, and we cannot rely on LLVM.

.. _constant-folding:

Constant Folding Pass
---------------------

There is a constant folding (also called constant propagation) pass done, before all the other passes. This
helps arithmetic of large types, and also means that the functions are constant folded when their arguments
are constant. For example:


.. code-block:: javascript

    bytes32 hash = keccak256('foobar');

This is evaluated at compile time. You can see this in the Visual Studio Code extension by hover over `hash`;
the hover will tell you the value of the hash.

.. _strength-reduce:

Strength Reduction Pass
-----------------------

Strength reduction is when expensive arithmetic is replaced with cheaper ones. So far, the following types
of arithmetic may be replaced:

- 256 or 128 bit multiply maybe replaced by 64 bit multiply or shift
- 256 or 128 bit divide maybe replaced by 64 bit divide or shift
- 256 or 128 bit modulo maybe replaced by 64 bit modulo or bitwise and

.. code-block:: javascript

    contract test {
        function f() public {
            for (uint i = 0; i < 10; i++) {
                // this multiply can be done with a 64 bit instruction
                g(i * 100));
            }
        }

        function g(uint256 v) internal {
            // ...
        }
    }

Solang uses reaching definitions to track the known bits of the variables; here solang knows that i can have
the values 0, 1, 2, 3, 4, 5, 6, 7, 8, 9 and the other operand is always 100. So, the multiplication can be
done using a single 64 bit multiply instruction. If you hover over the ``*`` in the Visual Studio Code you
will see this noted.

.. _dead-storage:

Dead Storage pass
-----------------

Loading from contract storage, or storing to contract storage is expensive. This optimization removes any
redundant load from and store to contract storage. If the same variable is read twice, then the value from
the first load is re-used. Similarly, if there is are two successive stores to the same variable, the first
one is removed as it is redundant. For example:

.. code-block:: javascript

    contract test {
        int a;

        // this function reads a twice; this can be reduced to one load
        function redundant_load() public returns (int) {
            return a + a;
        }

        // this function writes to contract storage thrice. This can be reduced to one
        function redundant_store() public {
            delete a;
            a = 1;
            a = 2;
        }
    }

This optimization pass can be disabled by running `solang --no-dead-storage`. You can see the difference between
having this optimization pass on by comparing the output of `solang --no-dead-storage --emit cfg foo.sol` with
`solang --emit cfg foo.sol`.

.. _vector-to-slice:

Vector to Slice Pass
--------------------

A `bytes` or `string` variable can be stored in a vector, which is a modifyable in-memory buffer, and a slice
which is a pointer to readonly memory and an a length. Since a vector is modifyable, each instance requires
a allocation. For example:

.. code-block:: javascript

    contract test {
        function can_be_slice() public {
            // v can just be a pointer to constant memory and an a length indicator
            string v = "Hello, World!";

            print(v);
        }

        function must_be_vector() public {
            // if v is a vector, then it needs to allocated and default value copied.
            string v = "Hello, World!";

            // bs is copied by reference is now modifyable
            bytes bs = v;


            bs[1] = 97;

            print(v);
        }
    }

This optimization pass can be disabled by running `solang --no-vector-to-slice`. You can see the difference between
having this optimization pass on by comparing the output of `solang --no-vector-to-slice --emit cfg foo.sol` with
`solang --emit cfg foo.sol`.

.. _unused-variable-elimination:

Unused Variable Elimination
----------------------------


During the semantic analysis, Solang detects unused variables and raises warnings for them.
During codegen, we remove all assignments that have been made to this unused variable. There is an example below:

.. code-block:: javascript

    contract test {

        function test1(int a) public pure returns (int) {
            int x = 5;
            x++;
            if (a > 0) {
                x = 5;
            }

            a = (x=3) + a*4;

            return a;
        }
    }

The variable 'x' will be removed from the function, as it has never been used. The removal won't affect any
expressions inside the function.

.. _common-subexpression-elimination:

Common Subexpression Elimination
---------------------------------


Solang performs common subexpression elimination by doing two passes over the CFG (Control
Flow Graph). During the first one, it builds a graph to track existing expressions and detect repeated ones.
During the second pass, it replaces the repeated expressions by a temporary variable, which assumes the value
of the expression. To disable this feature, use `solang --no-cse`.

Check out the example below. It contains multiple common subexpressions:

.. code-block:: javascript

     contract test {

         function csePass(int a, int b) public pure returns (int) {
             int x = a*b-5;
             if (x > 0) {
                 x = a*b-19;
             } else {
                 x = a*b*a;
             }

             return x+a*b;
         }
     }

The expression `a*b` is repeated throughout the function and will be saved to a temporary variable.
This temporary will be placed wherever there is an expression `a*b`. You can see the pass in action when you compile
this contract and check the CFG, using `solang --emit cfg`.
