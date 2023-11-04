Interacting with smart contracts on the command line
====================================================
Solang Aqd (عَقد, meaning "contract" in Arabic), is a Command-Line Interface (CLI) tool 
designed for easy interaction with smart contracts on Solana and Polkadot blockchains.
It simplifies the process of deploying smart contracts and executing specific functions on these contracts. 
This documentation provides essential guidance for using ``solang-aqd`` effectively.


Installation
____________

To install `solang-aqd`, run this command:

.. code-block:: bash

    cargo install --force --locked aqd
    
To update to the latest version, use the same command.


Submitting extrinsics to Polkadot on-chain
__________________________________________

The command line syntax for interacting with a program deployed on Polkadot is as follows:

aqd polkadot [SUBCOMMAND] [OPTIONS]... [FILE]

This means that the command line is ``aqd polkadot`` followed by a subcommand followed by any options described below,
followed by the filename. The filename could be ``.wasm file``, ``.contract`` bundle, or ``.json`` metadata file.

General Options (for all subcommands):
++++++++++++++++++++++++++++++++++++++

\-\-url *url*
  The websockets URL for the substrate node. [default: ws://localhost:9944]

\-\-network *network*
  Specify the network name to use.
  You can either specify a network name or a URL, but not both.

  Network:

  rococo
    Contracts (Rococo) (Equivalent to ``--url wss://rococo-contracts-rpc.polkadot.io``)

  phala-po-c5
    Phala PoC-5 (Equivalent to ``--url wss://poc5.phala.network/ws``)

  astar-shiden
    Astar Shiden (Kusama) (Equivalent to ``--url wss://rpc.shiden.astar.network``)

  astar-shibuya
    Astar Shibuya (Tokio) (Equivalent to ``--url wss://rpc.shibuya.astar.network``)

  astar
    Astar (Equivalent to ``--url wss://rpc.astar.network``)

  aleph-zero-testnet
    Aleph Zero Testnet (Equivalent to ``--url wss://ws.test.azero.dev``)

  aleph-zero
    Aleph Zero (Equivalent to ``--url wss://ws.azero.dev``)
  
  t3rnt0rn
    T3RN T0RN (Equivalent to ``--url wss://ws.t0rn.io``)
  
  pendulum-testnet
    Pendulum Testnet (Equivalent to ``--url wss://rpc-foucoco.pendulumchain.tech``)

-s, \-\-suri *suri*
  Specifies the secret key URI used for deploying the contract (must be specified). For example:
    For a development account: ``//Alice``
    
    With a password: ``//Alice///SECRET_PASSWORD``

-x, \-\-execute
  Specifies whether to submit the extrinsic for on-chain execution.

\-\-storage-deposit-limit *storage-deposit-limit*
  Specifies the maximum amount of balance that can be charged from the caller to pay for the storage consumed.

\-\-output-json
  Specifies whether to export the call output in JSON format.

\-\-help, -h
  This displays a short description of all the options

Subcommands:
++++++++++++

Upload Subcommand
-----------------

This subcommand enables the uploading of contracts onto the Polkadot blockchain.

.. code-block:: bash

  aqd polkadot upload --suri //Alice -x flipper.contract --output-json

Instantiate Subcommand
----------------------

This subcommand facilitates the instantiation of contracts on the Polkadot blockchain.

.. code-block:: bash

  aqd polkadot instantiate --suri //Alice --args true -x --output-json --skip-confirm flipper.contract

Options specific to the ``instantiate`` subcommand:
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

\-\-constructor *constructor*
  Specifies the name of the contract constructor to call. [default: new]

\-\-args *<args>...*
  Specifies the arguments of the contract constructor to call.

\-\-value *value*
  Specifies the value to be transferred as part of the call. [default: 0]

\-\-gas *gas*
  Specifies the maximum amount of gas to be used for this command.

\-\-proof-size *proof-size*
  Specifies the maximum proof size for this instantiation.

\-\-salt *salt*
  Specifies a salt used in the address derivation of the new contract.

-y, \-\-skip-confirm
  When set, skips the interactive confirmation prompt.

Call Subcommand
---------------

This subcommand enables the calling of contracts on the Polkadot blockchain."

.. code-block:: bash

  aqd polkadot call --contract 5EFYe3hkH2wFK1mLxD5VSqD88hfPZWihXAKeqozZELsL4Ueq --message get --suri //Alice flipper.contract --output-json --skip-confirm

Options specific to the ``call`` subcommand:
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

\-\-contract *contract*
  Specifies the address of the contract to call.

-m, \-\-message *message*
  Specifies the name of the contract message to call.

\-\-args *<args>...*
  Specifies the arguments of the contract message to call.

\-\-value *value*
  Specifies the value to be transferred as part of the call. [default: 0]

\-\-gas *gas*
  Specifies the maximum amount of gas to be used for this command.

\-\-proof-size *proof-size*
  Specifies the maximum proof size for this call.

-y, \-\-skip-confirm
  When set, skips the interactive confirmation prompt.

Remove Subcommand
-----------------

This subcommand allows for the removal of contracts from the Polkadot blockchain.

.. code-block:: bash

  aqd polkadot remove --suri //Alice --output-json --code-hash 0x94e67200d3d8f0f420873f8d1b426fdf5eb87f208c6e5d061822e017ffaef2a8 flipper.contract

Options specific to the ``remove`` subcommand:
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

\-\-code-hash *code_hash*
  Specifies the code hash to remove.


Interacting with Solana running programs on-chain
_________________________________________________

The command line syntax for interacting with a program deployed on Solana is as follows:

  aqd solana [SUBCOMMAND] [OPTIONS]...

It consists of a subcommand followed by its options, both of which are described below. 

.. note::

  Solang Aqd relies on the local default Solana configuration file to obtain information for transaction submissions. 
  For comprehensive management of this configuration file, you can refer to `Solana's CLI command documentation <https://docs.solana.com/cli/usage#solana-config>`_.

General Options (for all subcommands):
++++++++++++++++++++++++++++++++++++++

\-\-output-json
  Specifies whether to export the call output in JSON format.

\-\-help, -h
  This displays a short description of all the options.

Subcommands:
++++++++++++

Deploy Subcommand
------------------

Allows you to deploy Solana compiled programs to Solana.

.. code-block:: bash

  aqd solana deploy flipper.so


Show Subcommand
---------------

Show information about a Solana program's instructions given an IDL JSON file.

.. code-block:: bash

  aqd solana show --idl flipper.json --instruction new 

Options specific to the ``show`` subcommand:
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

\-\-idl *idl-json-file-path*
  Specifies the path of the IDL JSON file.

\-\-instruction *instruction-name*
  Specifies the name of the instruction to show information about.
  If not specified, information about all instructions is shown.

Call Subcommand
---------------

Send a custom transaction to a Solana program.

.. code-block:: bash

  aqd solana call --idl flipper.json --program G2eBnLvwPCGCFVywrUT2LtKCCYFkGetAVXJfW82UXmPe --instruction new --data true --accounts new self system

Options specific to the ``show`` subcommand:
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

\-\-idl *idl-json-file-path*
  Specifies the path of the IDL JSON file.

\-\-program *program*
  Specifies the program ID of the deployed program.

\-\-instruction *instruction-name*
  Specifies the name of the instruction to show information about.
  If not specified, information about all instructions is shown.

\-\-program *program*
  Specifies the program ID of the deployed program.

\-\-data *<data-arguments>...*
  Specifies the data arguments to pass to the instruction.
  For arrays and vectors, pass a comma-separated list of values. (e.g. 1,2,3,4).
  For structs, pass a JSON string of the struct. (can be a path to a JSON file).

\-\-accounts *<account-arguments>...*
  Specifies the accounts arguments to pass to the instruction

  Keywords:

  new
    Creates a new solana account and saves it locally. 

  self
    Reads the default keypair from the local configuration file.

  system
    Uses the system program ID as the account.

\-\-payer *payer*
  Specifies the payer keypair to use for the transaction. [default: local default keypair]
  