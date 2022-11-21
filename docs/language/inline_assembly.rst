Inline Assembly
===============

In Solidity functions, developers are allowed to write assembly blocks containing Yul code. For more information about
the Yul programming language, please refer to the :ref:`yul section <yul_section>`.

In an assembly block, you can access solidity local variables freely and modify them as well. Bear in mind, however,
that reference types like strings, vectors and structs are memory addresses in yul, so manipulating them can be unsafe
unless done correctly. Any assignment to those variables will change the address the reference points to and
may cause the program to crash if not managed correctly.

.. include:: ../examples/inline_assembly.sol
  :code: solidity

Storage variables cannot be accessed nor assigned directly. You must use the ``.slot`` and ``.offset`` suffix to use storage
variables. Storage variables should be read with the ``sload`` and saved with ``sstore`` builtins, but they are not implemented yet.
Solang does not implement offsets for storage variables, so the ``.offset`` suffix will always return zero.
Assignments to the offset are only allowed to Solidity local variables that are a reference to the storage.

.. include:: ../examples/inline_assembly_storage.sol
  :code: solidity

Dynamic calldata arrays should be accessed with the ``.offset`` and ``.length`` suffixes. The offset suffix returns the
array's memory address. Assignments to ``.length`` are not yet implemented.

.. include:: ../examples/inline_assembly_calldata.sol
  :code: solidity

External functions in Yul can be accessed and modified with the ``.selector`` and ``.address`` suffixes. The assignment
to those values, however, are not yet implemented.

.. include:: ../examples/inline_assembly_external_functions.sol
  :code: solidity
