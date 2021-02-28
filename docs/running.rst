Using Solang on the command line
================================

The Solang compiler is run on the command line. The solidity source file
names are provided as command line arguments; the output is an optimized
wasm or bpf file which is ready for deployment on a chain, and an abi file.

The following targets are supported right now:
`Parity Substrate <https://substrate.dev/>`_,
`Solana <https://www.solana.com/>`_
`Ethereum ewasm <https://github.com/ewasm/design>`_, and
`Sawtooth Sabre <https://github.com/hyperledger/sawtooth-sabre>`_.


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
  This takes one argument, which can either be ``ewasm``, ``sabre``, ``solana``,
  or ``substrate``. The default is substrate.

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

  ast
    Output Abstract Syntax Tree, the parsed and resolved input

  cfg
    Output control flow graph.

  llvm-ir
    Output llvm IR as text.

  llvm-bc
    Output llvm bitcode as binary file.

  object
    Output wasm object file; this is the contract before final linking.

Running Solang from docker image
________________________________

First pull the last Solang image from
`docker hub <https://hub.docker.com/repository/docker/hyperledgerlabs/solang/>`_:

.. code-block:: bash

    docker pull hyperledgerlabs/solang

And if you are using podman:

.. code-block:: bash

    podman image pull hyperlederlabs/solang

Now you can run Solang like so:

.. code-block:: bash

	  docker run --rm -it hyperledgerlabs/solang --version

Or podman:

.. code-block:: bash

	  podman container run --rm -it hyperledgerlabs/solang --version

If you want to compile some solidity files, the source file needs to be
available inside the container. You can do this via the -v command line.
In this example ``/local/path`` should be replaced with the absolute path
to your solidity files:

.. code-block:: bash

	  docker run --rm -it -v /local/path:/sources hyperledgerlabs/solang -o /sources /sources/flipper.sol

On podman you might need to add ``:Z`` to your volume argument if SELinux is used, like on Fedora. Also, podman allows relative paths:

.. code-block:: bash

	  podman container run --rm -it -v .:/sources:Z hyperledgerlabs/solang -o /sources /sources/flipper.sol

On Windows, you need to specify absolute paths:

.. code-block:: text

	docker run --rm -it -v C:\Users\User:/sources hyperledgerlabs/solang -o /sources /sources/flipper.sol
