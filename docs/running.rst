Using Solang on the command line
================================

The Solang compiler is run on the command line. The solidity source file
names are provided as command line arguments; the output is an optimized
wasm or bpf file which is ready for deployment on a chain, and an metadata
file (also known as the abi).

The following targets are supported right now:
`Solana <https://www.solana.com/>`_,
`Parity Substrate <https://substrate.dev/>`_, and
`Ethereum ewasm <https://github.com/ewasm/design>`_.


Usage
-----

Usage:

  solang [OPTIONS]... [SOLIDITY SOURCE FILE]...

This means that the command line is ``solang`` followed by any options described below,
followed by one or more solidity source filenames.

Options:

-v, \\-\\-verbose
  Make the output more verbose. The compiler tell you what contracts have been
  found in the source, and what files are generated. Without this option Solang
  will be silent if there are no errors or warnings.

\\-\\-target *target*
  This takes one argument, which can either be ``solana``, ``substrate``, or ``ewasm``. The target
  must be specified.

\\-\\-address\\-length *length-in-bytes*
  Change the default address length on Substrate. By default, Substate uses an address type of 32 bytes. This option
  is ignored for any other target.

\\-\\-value\\-length *length-in-bytes*
  Change the default value length on Substrate. By default, Substate uses an value type of 16 bytes. This option
  is ignored for any other target.

\\-\\-doc
  Generate documentation for the given Solidity files as a single html page. This uses the
  doccomment tags. The result is saved in ``soldoc.html``. See :ref:`tags` for
  further information.

-o, \\-\\-output *directory*
  This option takes one argument, which is the directory where output should
  be saved. The default is the current directory.

-O *optimization level*
  This takes one argument, which can either be ``none``, ``less``, ``default``,
  or ``aggressive``. These correspond to llvm optimization levels.

\\-\\-importpath *directory*
  When resolving ``import`` directives, search this directory. By default ``import``
  will only search the current directory. This option can be specified multiple times
  and the directories will be searched in the order specified.

\\-\\-importmap *map=directory*
  When resolving ``import`` directives, if the first part of the path matches *map*,
  search the directory provided for the file. This option can be specified multiple times
  with different values for map.

\\-\\-help, -h
  This displays a short description of all the options

\\-\\-standard-json
  This option causes Solang to emulate the behaviour of Solidity
  `standard json output <https://solidity.readthedocs.io/en/v0.5.13/using-the-compiler.html#output-description>`_. No output files are written, all the
  output will be in json on stdout.

  This feature is used by `Hyperledger Burrow's deploy tool <https://hyperledger.github.io/burrow/#/tutorials/3-contracts?id=deploy-artifacts>`_.

\\-\\-emit *phase*
  This option is can be used for debugging Solang itself. This is used to
  output early phases of compilation.

  Phase:

  ast-dot
    Output Abstract Syntax Tree as a graphviz dot file. This can be viewed with xdot
    or any other tool that can visualize graphviz dot files.

  cfg
    Output control flow graph.

  llvm-ir
    Output llvm IR as text.

  llvm-bc
    Output llvm bitcode as binary file.

  asm
    Output assembly text file.

  object
    Output wasm object file; this is the contract before final linking.

\\-\\-no\\-constant\\-folding
   Disable the :ref:`constant-folding` codegen optimization

\\-\\-no\\-strength\\-reduce
   Disable the :ref:`strength-reduce` codegen optimization

\\-\\-no\\-dead\\-storage
   Disable the :ref:`dead-storage` optimization

\\-\\-no\\-vector\\-to\\-slice
   Disable the :ref:`vector-to-slice` optimization

\\-\\-no\\-cse
   Disable the :ref:`common-subexpression-elimination` optimization

Running Solang using container
______________________________

First pull the last Solang container from
`solang containers <https://github.com/hyperledger-labs/solang/pkgs/container/solang>`_:

.. code-block:: bash

    docker pull ghcr.io/hyperledger-labs/solang

And if you are using podman:

.. code-block:: bash

    podman image pull ghcr.io/hyperledger-labs/solang

Now you can run Solang like so:

.. code-block:: bash

	  docker run --rm -it ghcr.io/hyperledger-labs/solang --version

Or podman:

.. code-block:: bash

	  podman container run --rm -it ghcr.io/hyperledger-labs/solang --version

If you want to compile some solidity files, the source file needs to be
available inside the container. You can do this via the -v command line.
In this example ``/local/path`` should be replaced with the absolute path
to your solidity files:

.. code-block:: bash

	  docker run --rm -it -v /local/path:/sources ghcr.io/hyperledger-labs/solang -o /sources /sources/flipper.sol

On Windows, you need to specify absolute paths:

.. code-block:: text

	docker run --rm -it -v C:\Users\User:/sources ghcr.io/hyperledger-labs/solang -o /sources /sources/flipper.sol
