Inline Assembly
===============

In Solidity functions, developers are allowed to write assembly blocks containing Yul code. For more information about
the Yul programming language, please refer to the :ref:`yul section <yul_section>`.

In an assembly block, you can access solidity local variables freely and modify them as well. Bear in mind, however,
that reference types like strings, vectors and structs are memory addresses in yul, so manipulating them can be unsafe
unless done correctly. Any assignment to those variables will change the address the reference points to and
may cause the program to crash if not managed correctly.

.. code-block:: solidity

    contract foo {
        struct test_stru {
            uint a;
            uint b;
        }

        function bar(uint64 a) public pure returns (uint64 ret) {
            uint64 b = 6;
            uint64[] memory vec;
            vec.push(4);
            string str = "cafe";
            test_stru tts = test_stru({a: 1, b: 2});
            assembly {
                // The following statements modify variables directly
                a := add(a, 3)
                b := mul(b, 2)
                ret := sub(a, b)

                // The following modify the reference address
                str := 5
                vec := 6
                tts := 7
            }

            // Any access to 'str', 'vec' or 'tts' here may crash the program.
        }

    }


Storage variables cannot be accessed nor assigned directly. You must use the ``.slot`` and ``.offset`` suffix to use storage
variables. Storage variables should be read with the ``sload`` and saved with ``sstore`` builtins, but they are not implemented yet.
Solang does not implement offsets for storage variables, so the ``.offset`` suffix will always return zero.
Assignments to the offset are only allowed to Solidity local variables that are a reference to the storage.

.. code-block:: solidity

    contract foo {
        struct test_stru {
            uint a;
            uint b;
        }

        test_stru storage_struct;
        function bar() public pure {
            test_stru storage tts = storage_struct;
            assembly {
                // The variables 'a' and 'b' contain zero
                let a := storage_struct.offset
                let b := tts.offset

                // This changes the reference slot of 'tts'
                tts.slot := 5
            }
        }
    }



Dynamic calldata arrays should be accessed with the ``.offset`` and ``.length`` suffixes. The offset suffix returns the
array's memory address. Assignments to ``.length`` are not yet implemented.

.. code-block:: solidity

    contract foo {
        function bar(int[] calldata vl) public pure {
            test_stru storage tts = storage_struct;
            assembly {
                // 'a' contains vl memory address
                let a := vl.offset

                // 'b' contains vl length
                let b := vl.length

                // This will change the reference of vl
                vl.offset := 5
            }
            // Any usage of vl here may crash the program
        }
    }


External functions in Yul can be accessed and modified with the ``.selector`` and ``.address`` suffixes. The assignment
to those values, however, are not yet implemented.

.. code-block:: solidity

    contract foo {
        function sum(uint64 a, uint64 b) public pure returns (uint64) {
            return a + b;
        }

        function bar() public view {
            function (uint64, uint64) external returns (uint64) fPtr = this.sum;
            assembly {
                // 'a' contains 'sum' selector
                let a := fPtr.selector

                // 'b' contains 'sum' address
                let b := vl.address
            }
        }
    }
