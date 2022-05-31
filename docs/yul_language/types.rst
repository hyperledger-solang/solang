Types
=====


The default type for variables in Yul is ``u256``. If there is no type specified, the compiler
will default to that integer type. Variables can have a type specified during their declaration using the following
syntax:


.. code-block:: yul

    {
        let a : u32, b : s64, d : u128, e : bool := multipleReturns()
    }

Yul allows signed and unsigned integer types, in addition to the boolean type. If a conversion from an integer to a
boolean is necessary, the compiler does the following operation ``number != 0``.

The unsigned types are the following: ``u8``, ``u32``, ``u64``, ``u128`` and ``u256``.

The signed types are the following: ``s8``, ``s32``, ``s64``, ``s128``, ``s256``.





