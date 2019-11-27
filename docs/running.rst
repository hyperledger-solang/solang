Running Solang
==============

The solang compiler is run on the command line. The solidity source file
names are provided as command line arguments; the output is an optimized
wasm file which is ready for deployment on a chain.

Two blockchains are supported right now:
`Hyperledger Burrow <https://github.com/hyperledger/burrow>`_ and
`Parity Substrate <https://substrate.dev/>`_.

.. note::

  Depending on which target solang is compiling for, different language
  features are supported. For example, when compiling for substrate, the
  constructor can be overloaded with different prototypes. With burrow, only
  one constructor prototype is allowed.

  The Solidity langauge has notes for each difference.
  
Using solang on the command line
--------------------------------

When running solang on the command line, the following command line options
are supported.

--target
  This takes one argument, which can either be ``burrow`` or ``substrate``.
  The default is substrate.

-o, --output
  This option takes one argument, which is the directory where output should
  be saved. The default is the current directory.
  
  .. FIXME this should be in the same directory as the solidity file

-O
  This takes one argument, which can either be ``none``, ``less``, ``default``,
  or ``aggressive``. These correspond to llvm optimization levels.

--help, -h
  This displays a short description of all the options


Using solang with Hyperledger Burrow
------------------------------------

This is documented in the `burrow documentation <https://hyperledger.github.io/burrow/#/reference/wasm>`_.
