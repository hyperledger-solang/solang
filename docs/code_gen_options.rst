Code Generation Options
=======================

There are compiler flags to control code generation. They can be divided into two categories: 

* Optimizer passes are enabled by default and make the generated code more optimal. 
* Debugging options are not enabled by default, but they greatly improve
  developer experience, at the cost of increased gas and storage usage.

Optimizer Passes
----------------

Solang generates its own internal IR, before the LLVM IR is generated. This internal IR allows us to do
several optimizations which LLVM cannot do, since it is not aware of higher-level language constructs.

Arithmetic of large integers (larger than 64 bit) has special handling, since LLVM cannot generate them.
So we need to do our own optimizations for these types, and we cannot rely on LLVM.

.. _constant-folding:

Constant Folding Pass
+++++++++++++++++++++

There is a constant folding (also called constant propagation) pass done, before all the other passes. This
helps arithmetic of large types, and also means that the functions are constant folded when their arguments
are constant. For example:


.. code-block:: solidity

    bytes32 hash = keccak256('foobar');

This is evaluated at compile time. You can see this in the Visual Studio Code extension by hover over `hash`;
the hover will tell you the value of the hash.

.. _strength-reduce:

Strength Reduction Pass
+++++++++++++++++++++++

Strength reduction is when expensive arithmetic is replaced with cheaper ones. So far, the following types
of arithmetic may be replaced:

- 256 or 128 bit multiply maybe replaced by 64 bit multiply or shift
- 256 or 128 bit divide maybe replaced by 64 bit divide or shift
- 256 or 128 bit modulo maybe replaced by 64 bit modulo or bitwise and

.. include:: ./examples/strength_reduce.sol
  :code: solidity

Solang uses reaching definitions to track the known bits of the variables; here solang knows that i can have
the values 0, 1, 2, 3, 4, 5, 6, 7, 8, 9 and the other operand is always 100. So, the multiplication can be
done using a single 64 bit multiply instruction. If you hover over the ``*`` in the Visual Studio Code you
will see this noted.

.. _dead-storage:

Dead Storage pass
+++++++++++++++++

Loading from contract storage, or storing to contract storage is expensive. This optimization removes any
redundant load from and store to contract storage. If the same variable is read twice, then the value from
the first load is re-used. Similarly, if there is are two successive stores to the same variable, the first
one is removed as it is redundant. For example:

.. include:: ./examples/dead_storage_elimination.sol
  :code: solidity

This optimization pass can be disabled by running `solang --no-dead-storage`. You can see the difference between
having this optimization pass on by comparing the output of `solang --no-dead-storage --emit cfg foo.sol` with
`solang --emit cfg foo.sol`.

.. _vector-to-slice:

Vector to Slice Pass
++++++++++++++++++++

A `bytes` or `string` variable can be stored in a vector, which is a modifyable in-memory buffer, and a slice
which is a pointer to readonly memory and an a length. Since a vector is modifyable, each instance requires
a allocation. For example:

.. include:: ./examples/vector_to_slice_optimization.sol
  :code: solidity

This optimization pass can be disabled by running `solang --no-vector-to-slice`. You can see the difference between
having this optimization pass on by comparing the output of `solang --no-vector-to-slice --emit cfg foo.sol` with
`solang --emit cfg foo.sol`.

.. _unused-variable-elimination:

Unused Variable Elimination
+++++++++++++++++++++++++++


During the semantic analysis, Solang detects unused variables and raises warnings for them.
During codegen, we remove all assignments that have been made to this unused variable. There is an example below:

.. include:: ./examples/unused_variable_elimination.sol
  :code: solidity

The variable 'x' will be removed from the function, as it has never been used. The removal won't affect any
expressions inside the function.

.. _common-subexpression-elimination:

Common Subexpression Elimination
++++++++++++++++++++++++++++++++


Solang performs common subexpression elimination by doing two passes over the CFG (Control
Flow Graph). During the first one, it builds a graph to track existing expressions and detect repeated ones.
During the second pass, it replaces the repeated expressions by a temporary variable, which assumes the value
of the expression. To disable this feature, use `solang --no-cse`.

Check out the example below. It contains multiple common subexpressions:

.. include:: ./examples/common_subexpression_elimination.sol
  :code: solidity

The expression `a*b` is repeated throughout the function and will be saved to a temporary variable.
This temporary will be placed wherever there is an expression `a*b`. You can see the pass in action when you compile
this contract and check the CFG, using `solang --emit cfg`.

.. _Array-Bound-checks-optimizations:

Array Bound checks optimization
+++++++++++++++++++++++++++++++

Whenever an array access is done, there must be a check for ensuring we are not accessing
beyond the end of an array. Sometimes, the array length could be known. For example:

.. include:: ./examples/array_bounds_check_optimization.sol
  :code: solidity

In this example we access ``array`` element 1, while the array length is 3. So, no bounds
checks are necessary and the code will more efficient if we do not emit the bounds check in
the compiled contract.

The array length is tracked in an invisible temporary variable, which is always kept up to date when, for example, a ``.pop()`` or ``.push()`` happens on the array
or an assignment happens. Then, when the bounds check happens, rather than retrieving the array length from
the array at runtime, bounds check becomes the constant expression `1 < 3` which is
always true, so the check is omitted.

This also means that, whenever the length of an array is accessed using '.length', it is replaced with a constant.

Note that this optimization does not cover every case. When an array is passed
as a function argument, for instance, the length is unknown.

Debugging Options
-----------------

It may be desirable to have access to debug information regarding the contract execution. 
However, this will lead to more instructions as well as higher gas usage. Hence, it is disabled by default,
but can be enabled using CLI flags. Just make sure not to use them in production deployments.

.. _log-api-return-codes:

Log runtime API call results
++++++++++++++++++++++++++++

Runtime API calls are not guaranteed to succeed.
By design, the low level results of these calls are abstracted away in Solidity.
For development purposes, it can be desirable to observe the low level return code of such calls.
The ``--log-api-return-codes`` flag will make the contract print the return code of runtime calls, if there are any.

.. note::

    This is only implemented for the Substrate target.

