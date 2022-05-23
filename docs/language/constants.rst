Constants
==========

Constants can be declared at the global level or at the contract level, just like contract
storage variables. They do not use any contract storage and cannot be modified.
The variable must have an initializer, which must be a constant expression. It is
not allowed to call functions or read variables in the initializer:

.. code-block:: javascript

    string constant greeting = "Hello, World!";

    contract ethereum {
        uint constant byzantium_block = 4_370_000;
    }

