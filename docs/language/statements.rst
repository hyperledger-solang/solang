Statements
==========

In functions, you can declare variables in code blocks. If the name is the same as
an existing function, enum type, or another variable, then the compiler will shadow
the original item and generate a warning as it is no longer accessible.

.. include:: ../examples/shadowing.sol
  :code: solidity

Scoping rules apply as you would expect, so if you declare a variable in a block, then it is not
accessible outside that block. For example:

.. include:: ../examples/scoping.sol
  :code: solidity

If statement
____________

Conditional execution of a block can be achieved using an ``if (condition) { }`` statement. The
condition must evaluate to a ``bool`` value.

.. include:: ../examples/statement_if.sol
  :code: solidity

The statements enclosed by ``{`` and ``}`` (commonly known as a *block*) are executed only if
the condition evaluates to true.

You can optionally add an ``else`` block which is executed only if the condition evaluates to false.

.. include:: ../examples/statement_if_else.sol
  :code: solidity

While statement
_______________

Repeated execution of a block can be achieved using ``while``. It syntax is similar to ``if``,
however the block is repeatedly executed until the condition evaluates to false.
If the condition is not true on first execution, then the loop body is never executed:

.. include:: ../examples/statement_while.sol
  :code: solidity

It is possible to terminate execution of the while statement by using the ``break`` statement.
Execution will continue to next statement in the function. Alternatively, ``continue`` will
cease execution of the block, but repeat the loop if the condition still holds:

.. include:: ../examples/statement_while_break.sol
  :code: solidity

Do While statement
__________________

A ``do { ... } while (condition);`` statement is much like the ``while (condition) { ... }`` except
that the condition is evaluated after executing the block. This means that the block is always executed
at least once, which is not true for ``while`` statements:

.. include:: ../examples/statement_do_while.sol
  :code: solidity

For statements
______________

For loops are like ``while`` loops with added syntaxic sugar. To execute a loop, we often
need to declare a loop variable, set its initial variable, have a loop condition, and then
adjust the loop variable for the next loop iteration.

For example, to loop from 0 to 1000 by steps of 100:

.. include:: ../examples/statement_for.sol
  :code: solidity

The declaration ``uint i = 0`` can be omitted if no new variable needs to be declared, and
similarly the post increment ``i += 100`` can be omitted if not necessary. The loop condition
must evaluate to a boolean, or it can be omitted completely. If it is omitted the block must
contain a ``break`` or ``return`` statement, else execution will
repeat infinitely (or until all gas is spent):

.. include:: ../examples/statement_for_abort.sol
  :code: solidity

.. _destructuring:

Destructuring Statement
_______________________

The destructuring statement can be used for making function calls to functions that have
multiple return values. The list can contain either:

1. The name of an existing variable. The type must match the type of the return value.
2. A new variable declaration with a type. Again, the type must match the type of the return value.
3. Empty; this return value is ignored and not accessible.

.. include:: ../examples/statement_destructing.sol
  :code: solidity

The right hand side may also be a list of expressions. This type can be useful for swapping
values, for example.

.. include:: ../examples/statement_destructing_swapping.sol
  :code: solidity

The right hand side of an destructure may contain the ternary conditional operator. The number
of elements in both sides of the conditional must match the left hand side of the destructure statement.

.. include:: ../examples/statement_destructing_conditional.sol
  :code: solidity

.. _try-catch:

Try Catch Statement
___________________

Solidity's try-catch statement can only be used with external calls or constructor calls using ``new``. The
compiler will refuse to compile any other expression.

Sometimes execution gets reverted due to a ``revert()`` or ``require()``. These types of problems
usually cause the entire transaction to be aborted. However, it is possible to catch
some of these problems in the caller and continue execution.

This is only possible for contract instantiation through new, and external function calls.
An internal function cannot be called from a try catch statement. Not all problems can be handled,
for example, out of gas cannot be caught. The ``revert()`` and ``require()`` builtins may
be passed a reason code, which can be inspected using the ``catch Error(string)`` syntax.

.. warning::
    On Solana, any transaction that fails halts the execution of a contract. The try-catch statement, thus,
    is not supported for Solana contracts and the compiler will raise an error if it detects its usage.

.. include:: ../examples/polkadot/statement_try_catch_constructor.sol
  :code: solidity

The same statement can be used for calling external functions. The ``returns (...)``
part must match the return types for the function. If no name is provided, that
return value is not accessible.

.. include:: ../examples/polkadot/statement_try_catch_call.sol
  :code: solidity

There is an alternate syntax which avoids the abi decoding by leaving the `catch Error(…)` out.
This might be useful when no error string is expected, and will generate shorter code.

.. include:: ../examples/polkadot/statement_try_catch_no_error_handling.sol
  :code: solidity

.. note::

    Try-catch only supports ``Error`` and ``Panic`` errors with an explicit catch clause.
    Calls reverting with a `custom error <https://docs.soliditylang.org/en/latest/abi-spec.html#errors>`_
    will be caught in the catch-all clause (``catch (bytes raw)``) instead.
    If there is no catch-all clause, custom errors will bubble up to the caller.
