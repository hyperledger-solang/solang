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

Using solang on the command line
--------------------------------

Usage:

  solang [OPTIONS]... [SOLIDITY SOURCE FILE]...

This means that the command line is ``solang`` followed by any options described below,
followed by one or more solidity source filenames.

Options:

-v, \\-\\-verbose
  Make the output more verbose. The compiler tell you what contracts have been
  found in the source, and what files are generated. Without this option solang
  will be silent if there are no errors.

\\-\\-target *target*
  This takes one argument, which can either be ``burrow`` or ``substrate``.
  The default is substrate.

-o, \\-\\-output *directory*
  This option takes one argument, which is the directory where output should
  be saved. The default is the current directory.

-O *optimization level*
  This takes one argument, which can either be ``none``, ``less``, ``default``,
  or ``aggressive``. These correspond to llvm optimization levels.

\\-\\-help, -h
  This displays a short description of all the options

\\-\\-standard-json
  This option causes solang to emulate the behaviour of Solidity
  `standard json output <https://solidity.readthedocs.io/en/v0.5.13/using-the-compiler.html#output-description>`_. No output files are written, all the
  output will be in json on stdout.

  This feature is used by `Hyperledger Burrow's deploy tool <https://hyperledger.github.io/burrow/#/tutorials/3-contracts?id=deploy-artifacts>`_.

  This setting implies ``--target burrow``.

\\-\\-emit *phase*
  This option is can be used for debugging solang itself. This is used to
  output early phases of compilation.

  Phase:

  cfg
    Output control flow graph.

  llvm
    Output llvm IR as text.

  bc
    Output llvm bitcode as binary file.

  object
    Output wasm object file; this is the contract before final linking.

Running solang from docker image
________________________________

First pull the last solang image from
`docker hub <https://hub.docker.com/repository/docker/hyperledgerlabs/solang/>`_::

        docker pull hyperledgerlabs/solang

And if you are using podman::

        podman image pull hyperlederlabs/solang

Now you can run solang like so::

	docker run --rm -it solang --version

Or podman::

	podman container run --rm -it solang --version

Now if you want to compile some solidity, the source file needs to be available
to the container. You can do this via the -v command line. ``/local/path`` should be replaced with the path to your solidity files::

	docker run --rm -it -v /local/path:/sources solang -o /sources /sources/contract.sol

On podman you might need to add ``:Z`` to your volume argument if SELinux is used, like on Fedora::

	podman container run --rm -it -v /local/path:/sources:Z solang -o /sources /sources/contract.sol

Using solang with Substrate
---------------------------

Solang builds contracts for Substrate by default. There is an solidity example
which can be found in the `examples <https://github.com/hyperledger-labs/solang/tree/master/examples>`_ directory::

  contract flipper {
  	bool private value;

  	constructor(bool initvalue) public {
  		value = initvalue;
  	}

  	function flip() public {
  		value = !value;
  	}

  	function get() public view returns (bool) {
  		return value;
  	}
  }

Write this to flipper.sol and run::

  solang flipper.sol

Now you should have ``flipper.wasm`` and ``flipper.json``. This can be used
directly in the `Polkadot UI <https://substrate.dev/substrate-contracts-workshop/#/0/deploying-your-contract?id=putting-your-code-on-the-blockchain>`_, as if the contract was written in Ink!.

Using solang with Hyperledger Burrow
------------------------------------

This is documented in the `burrow documentation <https://hyperledger.github.io/burrow/#/reference/wasm>`_.
