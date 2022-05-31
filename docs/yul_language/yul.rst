Overview of Yul
===============

.. _yul_section:

Yul, also know as EVM assembly, is a low level language for creating smart contracts and for providing more
control over the execution environment when using Solidity as the primary language. Although it enables
more possibilities to manage memory, using Yul does not imply a performance improvement. In Solang,
all Yul constructs are processed using the same pipeline as Solidity.

The support for Yul is only partial. We support all statements, except for the switch-block. In addition,
some Yul builtins are not yet implemented. In the following sections,
we'll describe the state of the compatibility of Yul in Solang.

