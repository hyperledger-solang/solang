File Structure
==============

A single Solidity source file may define multiple contracts. A contract is defined
with the ``contract`` keyword, following by the contract name and then the definition
of the contract in between curly braces ``{`` and ``}``.

.. code-block:: solidity

    contract A {
        /// foo simply returns true
        function foo() public returns (bool) {
            return true;
        }
    }

    contract B {
        /// bar simply returns false
        function bar() public returns (bool) {
            return false;
        }
    }

When compiling this, Solang will output contract code for both `A` and `B`, irrespective of
the name of source file. Although multiple contracts maybe defined in one solidity source
file, it might be convenient to define only single contract in each file, and keep contract
name the same as the file name (with the `.sol` extension).