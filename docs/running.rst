Running Solang
==============

The Solang compiler is run on the command line. The solidity source file
names are provided as command line arguments; the output is an optimized
wasm file which is ready for deployment on a chain, and an abi file.

The following targets are supported right now:
`Ethereum ewasm <https://github.com/ewasm/design>`_,
`Parity Substrate <https://substrate.dev/>`_,
`Solana <https://www.solana.com/>`_ and
`Sawtooth Sabre <https://github.com/hyperledger/sawtooth-sabre>`_.

.. note::

  Depending on which target Solang is compiling for, different language
  features are supported. For example, when compiling for substrate, the
  constructor can be overloaded with different prototypes. When targetting
  ewasm or Sawtooth Sabre, only one constructor prototype is allowed.

Using Solang on the command line
--------------------------------

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
  Generate documentation for the given Solidity as a simple html page. This uses the
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
  and they will be searched in-order.

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

  llvm
    Output llvm IR as text.

  bc
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

.. code-block::

	docker run --rm -it -v C:\Users\User:/sources hyperledgerlabs/solang -o /sources /sources/flipper.sol


Using Solang with Substrate
---------------------------

Solang builds contracts for Substrate by default. There is an solidity example
which can be found in the `examples <https://github.com/hyperledger-labs/solang/tree/master/examples>`_
directory. Write this to flipper.sol and run:

.. code-block:: bash

  solang --target substrate flipper.sol

Now you should have ``flipper.wasm`` and ``flipper.json``. This can be used
directly in the `Polkadot UI <https://substrate.dev/substrate-contracts-workshop/#/0/deploying-your-contract?id=putting-your-code-on-the-blockchain>`_, as if the contract was written in ink!.

Using solang with Solana
------------------------

The `Solana <https://www.solana.com/>`_ target is new and is limited right now, not all types are implemented
and other functionality is incomplete. However, the
`flipper example <https://github.com/hyperledger-labs/solang/tree/master/examples/flipper.sol>`_
can be used.

.. code-block:: bash

  solang --target solana flipper.sol -v

This will produce two files called `flipper.abi` and `flipper.so`. The first is an ethereum style abi file and the latter being
the ELF BPF shared object which can be deployed on Solana.

Solana has execution model which allows one program to interact with multiple accounts. Those accounts can
be used for different purposes. In Solang's case, each time the contract is executed, it needs two accounts.
The first account is for the `return data`, i.e. either the ABI encoded
return values or the revert buffer. The second account is to hold the contract storage variables.

Before any function on a smart contract can be used, the constructor must be first be called. This ensures that
the constructor as declared in the solidity code is executed, and that the contract storage account is
correctly initialized. To call the constructor, abi encode (using ethereum abi encoding) the constructor
arguments, and pass in two accounts to the call, the 2nd being the contract storage account.

Once that is done, any function on the contract can be called. To do that, abi encode the function call,
pass this as input, and provide two accounts on the call. The second account must be the same contract
storage account as used in the constructor. If there are any return values for the function, they
are stored in the first return data account. The first 8 bytes is a 64 bits length, followed by the
data itself. You can pass this into an ethereum abi decoder to get the expected return values.

There is `an example of this written in node <https://github.com/hyperledger-labs/solang/tree/master/integration/solana>`_.

Using Solang with Sawtooth Sabre
--------------------------------

When using Solang on Sawtooth Sabre, the constructor and function calls must be encoded with Ethereum ABI encoding.
This can be done in different ways. In this guide we use `ethabi <https://github.com/paritytech/ethabi>`_. This can
be installed using cargo:

.. code-block:: bash

  cargo install ethabi-cli

In order to abi encode the calls, we need the abi for the contract. Let's compile flipper.sol for Sabre:

.. code-block:: bash

  solang --target sabre --verbose flipper.sol

We now have a file ``flipper.wasm`` and ``flipper.abi``. To deploy this, we need to create the constructor
ABI encoding. Unfortunately ethabi already falls short here; we cannot encode constructor calls using the cli
tools. However we can work round this by specify the constructor arguments explicitly. Note that if the
constructor does not take any arguments, then the constructor data should be empty (0 bytes). So, since the
constructor in flipper.sol takes a single bool, create it like so:

.. code-block:: bash

  ethabi encode params -v bool true | xxd -r -p > constructor

For flipping the value, create it so:

.. code-block:: bash

  ethabi encode function flipper.abi flip | xxd -r -p  > flip

You'll also need a yaml file with the following contents. Save it to flipper.yaml.

.. code-block:: yaml

  name: flipper
  version: '1.0'
  wasm: flipper.wasm
  inputs:
  - '12cd3c'
  outputs:
  - '12cd3c'

Now we have to start the Sawtooth Sabre environment. First clone the
`Sawtooth Sabre github repo <https://github.com/hyperledger/sawtooth-sabre/>`_ and then run:

.. code-block:: bash

  docker-compose -f docker-compose-installed.yaml up --build

Now enter the sabre-cli container:

.. code-block:: bash

  docker exec -it sabre-cli bash

To create the flipper contract, run the following:

.. code-block:: bash

  sabre cr --create flipper --owner $(cat /root/.sawtooth/keys/root.pub) --url http://rest-api:9708
  sabre upload --filename flipper.yaml --url http://rest-api:9708
  sabre ns --create 12cd3c --url http://rest-api:9708 --owner $(cat /root/.sawtooth/keys/root.pub)
  sabre perm 12cd3c flipper --read --write --url http://rest-api:9708

To run the constructor, run:

.. code-block:: bash

   sabre exec --contract flipper:1.0 --payload  ./constructor --inputs 12cd3c  --outputs 12cd3c --url http://rest-api:9708

Lastly, to run the flip function:

.. code-block:: bash

  sabre exec --contract flipper:1.0 --payload  ./flip --inputs 12cd3c  --outputs 12cd3c --url http://rest-api:9708

.. warning::

  Returning values from Solidity is not yet implemented, and neither is ``revert()``. If you
  attempt to call a function which returns a value, it will fail.

Using Solang with Hyperledger Burrow
------------------------------------

In Burrow, Solang is used transparently by the ``burrow deploy`` tool if it is given the ``--wasm`` argument.
When building and deploying a Solidity contract, rather than running the ``solc`` compiler, it will run
the ``solang`` compiler and deploy it as a wasm contract.

This is documented in the `burrow documentation <https://hyperledger.github.io/burrow/#/reference/wasm>`_.
