Statements
==========

In functions, you can declare variables in code blocks. If the name is the same as
an existing function, enum type, or another variable, then the compiler will shadow
the original item and generate a warning as it is no longer accessible.

.. code-block:: solidity

  contract test {
      uint foo = 102;
      uint bar;

      function foobar() private {
          // AVOID: this shadows the contract storage variable foo
          uint foo = 5;
      }
  }

Scoping rules apply as you would expect, so if you declare a variable in a block, then it is not
accessible outside that block. For example:

.. code-block:: solidity

   function foo() public {
      // new block is introduced with { and ends with }
      {
          uint a;

          a = 102;
      }

      // ERROR: a is out of scope
      uint b = a + 5;
  }

If statement
____________

Conditional execution of a block can be achieved using an ``if (condition) { }`` statement. The
condition must evaluate to a ``bool`` value.

.. code-block:: solidity

  function foo(uint32 n) private {
      if (n > 10) {
          // do something
      }

      // ERROR: unlike C integers can not be used as a condition
      if (n) {
            // ...
      }
  }

The statements enclosed by ``{`` and ``}`` (commonly known as a *block*) are executed only if
the condition evaluates to true.

You can optionally add an ``else`` block which is executed only if the condition evaluates to false.

.. code-block:: solidity

    function foo(uint32 n) private {
        if (n > 10) {
            // do something
        } else {
            // do something different
        }
    }


While statement
_______________

Repeated execution of a block can be achieved using ``while``. It syntax is similar to ``if``,
however the block is repeatedly executed until the condition evaluates to false.
If the condition is not true on first execution, then the loop body is never executed:

.. code-block:: solidity

  function foo(uint n) private {
      while (n >= 10) {
          n -= 9;
      }
  }

It is possible to terminate execution of the while statement by using the ``break`` statement.
Execution will continue to next statement in the function. Alternatively, ``continue`` will
cease execution of the block, but repeat the loop if the condition still holds:

.. code-block:: solidity

  function foo(uint n) private {
      while (n >= 10) {
          n--;

          if (n >= 100) {
              // do not execute the if statement below, but loop again
              continue;
          }

          if (bar(n)) {
              // cease execution of this while loop and jump to the "n = 102" statement
              break;
          }

          // only executed if both if statements were false
          print("neither true");
      }

      n = 102;
  }

Do While statement
__________________

A ``do { ... } while (condition);`` statement is much like the ``while (condition) { ... }`` except
that the condition is evaluated after executing the block. This means that the block is always executed
at least once, which is not true for ``while`` statements:

.. code-block:: solidity

  function foo(uint n) private {
      do {
          n--;

          if (n >= 100) {
              // do not execute the if statement below, but loop again
              continue;
          }

          if (bar(n)) {
              // cease execution of this while loop and jump to the "n = 102" statement
              break;
          }
      }
      while (n > 10);

      n = 102;
  }

For statements
______________

For loops are like ``while`` loops with added syntaxic sugar. To execute a loop, we often
need to declare a loop variable, set its initial variable, have a loop condition, and then
adjust the loop variable for the next loop iteration.

For example, to loop from 0 to 1000 by steps of 100:

.. code-block:: solidity

  function foo() private {
      for (uint i = 0; i <= 1000; i += 100) {
          // ...
      }
  }

The declaration ``uint i = 0`` can be omitted if no new variable needs to be declared, and
similarly the post increment ``i += 100`` can be omitted if not necessary. The loop condition
must evaluate to a boolean, or it can be omitted completely. If it is ommited the block must
contain a ``break`` or ``return`` statement, else execution will
repeat infinitely (or until all gas is spent):

.. code-block:: solidity

  function foo(uint n) private {
      // all three omitted
      for (;;) {
          // there must be a way out
          if (n == 0) {
              break;
          }
      }
  }

.. _destructuring:

Destructuring Statement
_______________________

The destructuring statement can be used for making function calls to functions that have
multiple return values. The list can contain either:

1. The name of an existing variable. The type must match the type of the return value.
2. A new variable declaration with a type. Again, the type must match the type of the return value.
3. Empty; this return value is ignored and not accessible.

.. code-block:: solidity

    contract destructure {
        function func() internal returns (bool, int32, string) {
            return (true, 5, "abcd")
        }

        function test() public {
            string s;
            (bool b, , s) = func();
        }
    }

The right hand side may also be a list of expressions. This type can be useful for swapping
values, for example.

.. code-block:: solidity

    function test() public {
        (int32 a, int32 b, int32 c) = (1, 2, 3);

        (b, , a) = (a, 5, b);
    }

The right hand side of an destructure may contain the ternary conditional operator. The number
of elements in both sides of the conditional must match the left hand side of the destructure statement.

.. code-block:: javascript

    function test(bool cond) public {
        (int32 a, int32 b, int32 c) = cond ? (1, 2, 3) : (4, 5, 6)
    }


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

.. code-block:: solidity

    contract aborting {
        constructor() {
            revert("bar");
        }
    }

    contract runner {
        function test() public {
            try new aborting() returns (aborting a) {
                // new succeeded; a holds the a reference to the new contract
            }
            catch Error(string x) {
                if (x == "bar") {
                    // "bar" revert or require was executed
                }
            }
            catch (bytes raw) {
                // if no error string could decoding, we end up here with the raw data
            }
        }
    }

The same statement can be used for calling external functions. The ``returns (...)``
part must match the return types for the function. If no name is provided, that
return value is not accessible.

.. code-block:: solidity

    contract aborting {
        function abort() public returns (int32, bool) {
            revert("bar");
        }
    }

    contract runner {
        function test() public {
            aborting abort = new aborting();

            try abort.abort() returns (int32 a, bool b) {
                // call succeeded; return values are in a and b
            }
            catch Error(string x) {
                if (x == "bar") {
                    // "bar" reason code was provided through revert() or require()
                }
            }
            catch (bytes raw) {
                // if no error string could decoding, we end up here with the raw data
            }
        }
    }

There is an alternate syntax which avoids the abi decoding by leaving the `catch Error(…)` out.
This might be useful when no error string is expected, and will generate shorter code.

.. code-block:: solidity

    contract aborting {
        function abort() public returns (int32, bool) {
            revert("bar");
        }
    }

    contract runner {
        function test() public {
            aborting abort = new aborting();

            try new abort.abort() returns (int32 a, bool b) {
                // call succeeded; return values are in a and b
            }
            catch (bytes raw) {
                // call failed with raw error in raw
            }
        }
    }

