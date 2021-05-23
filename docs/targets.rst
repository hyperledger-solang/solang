Target Specific
===============


Parity Substrate
________________

Solang works with Parity Substrate 3.0. This target is the most mature and has received the most testing so far.

The Parity Substrate has the following differences to Ethereum Solidity:

- The address type is 32 bytes, not 20 bytes
- An address literal has to be specified using the ``address"5GBWmgdFAMqm8ZgAHGobqDqX6tjLxJhv53ygjNtaaAn3sjeZ"`` syntax
- ABI encoding and decoding is done using the `SCALE <https://substrate.dev/docs/en/overview/low-level-data-format>`_ encoding
- Multiple constructors are allowed, and can be overloaded

There is an solidity example which can be found in the
`examples <https://github.com/hyperledger-labs/solang/tree/main/examples>`_
directory. Write this to flipper.sol and run:

.. code-block:: bash

  solang --target substrate flipper.sol

Now you should have a file called ``flipper.contract``. The file contains both the ABI and contract wasm.
It can be used directly in the
`Polkadot UI <https://substrate.dev/substrate-contracts-workshop/#/0/deploying-your-contract?id=putting-your-code-on-the-blockchain>`_, as if the contract was written in ink!.


Solana
______

The Solana target requires `Solana <https://www.solana.com/>`_ 1.6.9 or later. This target is in the early stages right now,
however it is under active development. All data types are supported, but the builtin functions, and constructor calls
have not been implemented yet. This is how to build your Solidity for Solana:

.. code-block:: bash

  solang --target solana flipper.sol -v

This will produce two files called `flipper.abi` and `bundle.so`. The first is an ethereum style abi file and the latter being
the ELF BPF shared object which can be deployed on Solana. For each contract, an abi file will be created; a single `bundle.so`
is created which contains the code all the contracts provided on the command line.

The contract storage model in Solana is different from Ethereum; it is consists of a contigious piece of memory, which can be
accessed directly from the smart contract. This means that there are no `storage slots`, and that a `mapping` must be implemented
using a simple hashmap. The same hashmap is used for fixed-length arrays which are a larger than 1kb. So, if you declare an
contract storage array of ``int[10000]``, then this is implemented using a hashmap.

Solana has execution model which allows one program to interact with multiple accounts. Those accounts can
be used for different purposes. In Solang's case, each time the contract is executed, it needs two accounts.
One account is the program, which contains the compiled BPF program. The other account contains the contract storage
variables, and also the return variables for the last invocation.

The output of the compiler will tell you how large the second account needs to be. For the `flipper.sol` example,
the output contains *"info: contract flipper uses at least 17 bytes account data"*. This means the second account
should be 17 bytes plus space for the return data, and any dynamic storage. If the account is too small, the transaction
will fail with the error *account data too small for instruction*.

Before any function on a smart contract can be used, the constructor must be first be called. This ensures that
the constructor as declared in the solidity code is executed, and that the contract storage account is
correctly initialized. To call the constructor, abi encode (using ethereum abi encoding) the constructor
arguments, and pass in two accounts to the call, the 2nd being the contract storage account.

Once that is done, any function on the contract can be called. To do that, abi encode the function call,
pass this as input, and provide the two accounts on the call, plus any accounts that may be called. The return data may
be read from the account data if the call succeeds.

There is `an example of this written in node <https://github.com/hyperledger-labs/solang/tree/main/integration/solana>`_.

Hyperledger Burrow (ewasm)
__________________________

The ewasm specification is not finalized yet. There is no `create2` or `chainid` call, and the keccak256 precompile
contract has not been finalized yet.

In Burrow, Solang is used transparently by the ``burrow deploy`` tool if it is given the ``--wasm`` argument.
When building and deploying a Solidity contract, rather than running the ``solc`` compiler, it will run
the ``solang`` compiler and deploy it as a wasm contract.

This is documented in the `burrow documentation <https://hyperledger.github.io/burrow/#/reference/wasm>`_.

ewasm has been tested with `Hyperledger Burrow <https://github.com/hyperledger/burrow>`_.
Please use the latest master version of burrow, as ewasm support is still maturing in Burrow.

Some language features have not been fully implemented yet on ewasm:

- Contract storage variables types ``string``, ``bytes`` and function types are not implemented

Hyperledger Sawtooth
____________________

This is merely a proof-of-concept target, and has seen very little testing. Unless anyone is interested in
maintaining this target, it is likely to be removed. On sawtooth, many Solidity concepts are impossible to implement:

- Return values from contract calls
- Calling other contracts
- Value transfers
- Instantiating contracts

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

  For the Sawtooth target,
  returning values from Solidity is not yet implemented, and neither is ``revert()``. If you
  attempt to call a function which returns a value, it will fail.

