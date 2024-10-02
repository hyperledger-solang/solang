Solana
======

Solana Overview
_______________

As the underlying Solana environment is different than that of Ethereum, Solidity inner workings
have been modified to function properly. For example, A Solidity contract on Solana utilizes two accounts: a
data account and a program account. The program account stores the contract's executable binary and owns
the data account, which holds all the storage variables. On Ethereum a single account can store executable
code and data.

Contract upgrades
+++++++++++++++++

Provided that the data layout from a new contract is compatible with that of an old one, it is possible
to update the binary in the program account and retain the same data, rendering
contract upgrades implemented in Solidity unnecessary. Solana's CLI tool provides a command to both do
an initial deploy of a program, and redeploy it later.:

.. code-block:: bash

    solana program deploy --program-id <KEYPAIR_FILEPATH> <PROGRAM_FILEPATH>

where ``<KEYPAIR_FILEPATH>`` is the program's keypair json file and ``<PROGRAM_FILEPATH>``
is the program binary ``.so`` file. For more information about redeploying a program,
check `Solana's documentation <https://docs.solana.com/cli/deploy-a-program#redeploy-a-program>`_.

Data types
++++++++++

- An account address consists of a 32-bytes key, which is represented by the ``address`` type. This data model
  differs from Ethereum 20-bytes addresses.
- Solana's virtual machine registers are 64-bit wide, so 64-bit integers ``uint64`` and ``int64`` are preferable over
  ``uint256`` and ``int256``. An operation with types wider than 64-bits is split into multiple operations, making
  it slower and consuming more compute units. This is the case, for instance, with multiplication, division and modulo
  using `uint256`.
- Likewise, all balances and values on Solana are 64-bit wide, so the builtin functions for
  *address* ``.balance``, ``.transfer()`` and ``.send()`` use 64-bit integers.
- An address literal has to be specified using the ``address"36VtvSbE6jVGGQytYWSaDPG7uZphaxEjpJHUUpuUbq4D"`` syntax.
- Ethereum syntax for addresses ``0xE0f5206BBD039e7b0592d8918820024e2a7437b9`` is not supported.

Runtime
+++++++

- The Solana target requires `Solana <https://www.solana.com/>`_ v1.8.1.
- Function selectors are eight bytes wide and known as *discriminators*.
- Solana provides different builtins, e.g. ``block.slot`` and ``tx.accounts``.
- When calling an external function or invoking a contract's constructor, one
  :ref:`needs to provide <solana_cpi_accounts>` the necessary accounts for the transaction.
- The keyword ``this`` returns the contract's program account, also know as program id.
- Contracts :ref:`cannot be types <contracts_not_types>` on Solana and :ref:`calls to contracts <solana_contract_call>`
  follow a different syntax.
- Accounts can be declared on functions using :ref:`annotations <account_management>`.
- :ref:`Retrieving the balance <balance>` and :ref:`transferring values <send_transfer>`
  utilize the ``AccountInfo`` struct.


Compute budget
++++++++++++++

On Ethereum, when calling a smart contract function, one needs to specify the amount of gas the operation is allowed
to use. Gas serves to pay for a contract execution on chain and can be a way for giving a contract priority execution
when extra gas is offered in a transaction. Each EVM instruction has an associated gas value, which translates to real
ETH cost. Provided that one can afford all the gas expenses, there is no upper boundary for the amount of gas limit
one can provide in a transaction, so Solidity for Ethereum has gas builtins, like ``gasleft``, ``block.gaslimit``,
``tx.gasprice`` or the Yul ``gas()`` builtin, which returns the amount of gas left for execution.

On the other hand, Solana is optimized for low latency and high transaction throughput and has an equivalent concept to
gas: compute unit. Every smart contract function is allowed the same quantity of compute units (currently that
value is 200k), and every instruction of a contract consumes exactly one compute unit. There is no need to provide
an amount of compute units for a transaction and they are not charged, except when one wants priority execution on
chain, in which case one would pay per compute unit consumed. Therefore, functions for gas are not available on
Solidity for Solana.


Solidity for Solana incompatibilities with Solidity for Ethereum
++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++

- ``msg.sender`` is :ref:`not available on Solana <msg_sender_solana>`.
- There is no ``ecrecover()`` builtin function because Solana does not use the ECDSA algorithm, but there
  is a ``signatureVerify()`` function, which can check ed25519 signatures. As a consequence, it is not possible to
  recover a signer from a signature.
- Try-catch statements do not work on Solana. If any external call or contract creation fails, the runtime will
  halt execution and revert the entire transaction.
- Error definitions and reverts with error messages are not yet working for Solana.
- Value transfer with function call :ref:`does not work <value_transfer>`.
- Many Yul builtins are not available, as specified in the :ref:`availability table <yul_builtins>`.
- External calls on Solana require that accounts be specified, as in :ref:`this example <solana_external_call>`.
- The ERC-20 interface is not compatible with Solana at the moment.

Getting started on Solana
_________________________

Please follow the `Solang Getting Started Guide <https://solana.com/developers/guides/solang/getting-started>`_.

For more examples, see the
`solang's integration tests <https://github.com/hyperledger-solang/solang/tree/main/integration/solana>`_.

Using the Anchor client library
_______________________________

Some notes on using the anchor javascript npm library.

* Solidity function names are converted to camelCase. This means that if in Solidity a function is called ``foo_bar()``,
  you must write ``fooBar()`` in your javascript.
* Anchor only allows you to call ``.view()`` on Solidity functions which are declared ``view`` or ``pure``.
* Named return values in Solidity are also converted to camelCase. Unnamed returned are given the name ``return0``, ``return1``, etc,
  depending on the position in the returns values.
* Only return values from ``view`` and ``pure`` functions can be decoded. Return values from other functions and are not accessible.
  This is a limitation in the Anchor library. Possibly this can be fixed.
* In the case of an error, no return data is decoded. This means that the reason provided in ``revert('reason');`` is not
  available as a return value.
* Number arguments for functions are expressed as ``BN`` values and not plain javascript ``Number`` or ``BigInt``.

.. _call_anchor:

Calling Anchor Programs from Solidity
_____________________________________

It is possible to call `Anchor Programs <https://github.com/coral-xyz/anchor>`_
from Solidity. You first have to generate a Solidity interface file from the IDL file using
the :ref:`idl_command`. Then, import the Solidity file in your Solidity using the
``import "...";`` syntax. Say you have an anchor program called ``bobcat`` with a
function ``pounce``, you can call it like so:

.. include:: ../examples/solana/call_anchor.sol
  :code: solidity

Setting the program_id for a contract
_____________________________________

When developing contracts for Solana, programs are usually deployed to a well
known account. The account can be specified in the source code using an annotation
``@program_id`` if it is known beforehand. If you want to call a contract via an external call,
either the contract must have a ``@program_id`` annotation or the ``{program_id: ..}`` call argument must be present.

.. include:: ../examples/solana/program_id.sol
  :code: solidity

.. note::

    The program_id ``Foo5mMfYo5RhRcWa4NZ2bwFn4Kdhe8rNK5jchxsKrivA`` was generated using
    the command line:

    .. code-block:: bash

        solana-keygen grind --starts-with Foo:1

.. _payer_seeds_bump:

Setting the payer, seeds, bump, and space for a contract
_________________________________________________________

When a contract is instantiated, there are two accounts required: the program account to hold
the executable code and the data account to save the state variables of the contract. The
program account is deployed once and can be reused for updating the contract. When each
Solidity contract is instantiated (also known as deployed), the data account has to be
created. This can be done by the client-side code, and then the created blank account
is passed to the transaction that runs the constructor code.

Alternatively, the data account can be created by the constructor, on chain. When
this method is used, some parameters must be specified for the account
using annotations. Annotations placed above a constructor can only contain literals or
constant expressions, as is the case for first ``@seed`` and ``@space`` in the following example.
Annotations can also refer to constructor arguments when placed next to them, as the second ``@seed`` and
the ``@bump`` examples below. The ``@payer`` annotation is a special annotation that
:ref:`declares an account <account_management>`.

If the contract has no constructor, annotations can be paired with an empty constructor.

.. include:: ../examples/solana/constructor_annotations.sol
  :code: solidity

Creating an account needs a payer, so at a minimum the ``@payer`` annotation must be
specified. If it is missing, then the data account must be created client-side.
The ``@payer`` annotation :ref:`declares a Solana account <account_management>` that must be passed in the transaction.

The size of the data account can be specified with ``@space``. This is a
``uint64`` expression which can either be a constant or use one of the constructor
arguments. The ``@space`` should at least be the size given when you run ``solang -v``:

.. code-block:: bash

    $ solang compile --target solana -v examples/solana/flipper.sol
    ...
    info: contract flipper uses at least 17 bytes account data
    ...

If the data account is going to be a
`program derived address <https://docs.solana.com/developing/programming-model/calling-between-programs#program-derived-addresses>`_,
then the seeds and bump have to be provided. There can be multiple seeds, and an optional
single bump. If the bump is not provided, then the seeds must not create an
account that falls on the curve. When placed above the constructor, the ``@seed`` can be a string literal,
or a hex string with the format ``hex"4142"``. If before an argument, the seed annotation must refer to an argument
of type ``bytes``, ``address``, or fixed length byte array of ``bytesN``. The ``@bump`` must a single byte of type ``bytes1``.

.. _value_transfer:

Transferring native value with a function call
______________________________________________

The Solidity language on Ethereum allows value transfers with an external call
or constructor, using the ``auction.bid{value: 501}()`` syntax.
Solana Cross Program Invocation (CPI) does not support this, which means that:

 - Specifying ``value:`` on an external call or constructor is not permitted
 - The ``payable`` keyword has no effect
 - ``msg.value`` is not supported

.. note::

    A naive way to implement this is to let the caller transfer
    native balance and then inform the callee about the amount transferred by
    specifying this in the instruction data. However, it would be trivial to
    forge such an operation.

Receive function
________________

In Solidity the ``receive()`` function, when defined, is called whenever the native
balance for an account gets credited, for example through a contract calling
``account.transfer(value);``. On Solana, there is no method that implements
this. The balance of an account can be credited without any code being executed.

``receive()`` functions are not permitted on the Solana target.

.. _msg_sender_solana:

``msg.sender`` not available on Solana
______________________________________

On Ethereum, ``msg.sender`` is used to identify either the account that submitted
the transaction, or the caller when one contract calls another. On Ethereum, each
contract execution can only use a single account, which provides the code and data.
On Solana, each contract execution uses many accounts. Consider a rust contract which
calls a Solidity contract: the rust contract can access a few data accounts, and which
of those would be considered the caller? So in many cases there is not a single account
which can be identified as a caller. In addition to that, the Solana VM has no
mechanism for fetching the caller accounts. This means there is no way to implement
``msg.sender``.

The way to implement this on Solana is to have an authority account for the contract
that must be a signer for the transaction (note that on Solana there
can be many signers too). This is a common construct on Solana contracts.

.. include:: ../examples/solana/use_authority.sol
  :code: solidity

Builtin Imports
________________

Some builtin functionality is only available after importing. The following structs
can be imported via the special builtin import file ``solana``.

.. code-block:: solidity

    import {AccountMeta, AccountInfo} from 'solana';

Note that ``{AccountMeta, AccountInfo}`` can be omitted, renamed or imported via
import object.

.. code-block:: solidity

    // Now AccountMeta will be known as AM
    import {AccountMeta as AM} from 'solana';

    // Now AccountMeta will be available as solana.AccountMeta
    import 'solana' as solana;

.. note::

    The import file ``solana`` is only available when compiling for the Solana
    target.

.. _account_info:

Builtin AccountInfo
+++++++++++++++++++

The account info of all the accounts passed into the transaction. ``AccountInfo`` is a builtin
structure with the following fields:

address ``key``
    The address (or public key) of the account

uint64 ``lamports``
    The lamports of the accounts. This field can be modified, however the lamports need to be
    balanced for all accounts by the end of the transaction.

bytes ``data``
    The account data. This field can be modified, but use with caution.

address ``owner``
    The program that owns this account

uint64 ``rent_epoch``
    The next epoch when rent is due.

bool ``is_signer``
    Did this account sign the transaction

bool ``is_writable``
    Is this account writable in this transaction

bool ``executable``
    Is this account a program

.. _account_meta:

Builtin AccountMeta
+++++++++++++++++++

When doing an external call (aka CPI), ``AccountMeta`` specifies which accounts
should be passed to the callee.

address ``pubkey``
    The address (or public key) of the account

bool ``is_writable``
    Can the callee write to this account

bool ``is_signer``
    Can the callee assume this account signed the transaction

Builtin create_program_address
++++++++++++++++++++++++++++++

This function returns the program derived address for a program address and
the provided seeds. See the Solana documentation on
`program derived addresses <https://edge.docs.solana.com/developing/programming-model/calling-between-programs#program-derived-addresses>`_.

.. include:: ../examples/solana/builtin_create_program_address.sol
  :code: solidity

Builtin try_find_program_address
++++++++++++++++++++++++++++++++

This function returns the program derived address for a program address and
the provided seeds, along with a seed bump. See the Solana documentation on
`program derived addresses <https://edge.docs.solana.com/developing/programming-model/calling-between-programs#program-derived-addresses>`_.

.. include:: ../examples/solana/builtin_try_find_program_address.sol
  :code: solidity

Solana Library
______________

In Solang's Github repository, there is a directory called ``solana-library``. It contains libraries for Solidity contracts
to interact with Solana specific instructions. We provide two libraries: one for SPL tokens and another
for Solana's system instructions. In order to use those functionalities, copy the correspondent library
file to your project and import it.

SPL-token
+++++++++

`spl-token <https://spl.solana.com/token>`_ is the Solana native way of creating tokens, minting, burning and
transferring token. This is the Solana equivalent of
`ERC-20 <https://ethereum.org/en/developers/docs/standards/tokens/erc-20/>`_ and
`ERC-721 <https://ethereum.org/en/developers/docs/standards/tokens/erc-721/>`_. Solang's repository contains
a library ``SplToken`` to use spl-token from Solidity. The file
`spl_token.sol <https://github.com/hyperledger-solang/solang/blob/main/solana-library/spl_token.sol>`_  should be copied into
your source tree, and then imported in your solidity files where it is required. The ``SplToken`` library has doc
comments explaining how it should be used.

There is an example in our integration tests of how this should be used. See
`token.sol <https://github.com/hyperledger-solang/solang/blob/main/integration/solana/token.sol>`_ and
`token.spec.ts <https://github.com/hyperledger-solang/solang/blob/main/integration/solana/token.spec.ts>`_.


.. _system_instruction_library:

System Instructions
+++++++++++++++++++

Solana's system instructions enable developers to interact with Solana's System Program. There are functions to
create new accounts, allocate account data, assign accounts to owning programs, transfer lamports from System Program
owned accounts and pay transaction fees. More information about the functions offered can be found both on
`Solana documentation <https://docs.rs/solana-program/1.11.10/solana_program/system_instruction/enum.SystemInstruction.html>`_
and on Solang's `system_instruction.sol <https://github.com/hyperledger-solang/solang/blob/main/solana-library/system_instruction.sol>`_ file.

The usage of system instructions needs the correct setting of writable and signer accounts when interacting with Solidity
contracts on chain. Examples are available on Solang's integration tests.
See `system_instruction_example.sol <https://github.com/hyperledger-solang/solang/blob/main/integration/solana/system_instruction_example.sol>`_
and `system_instruction.spec.ts <https://github.com/hyperledger-solang/solang/blob/main/integration/solana/system_instruction.spec.ts>`_

Minimum balance
+++++++++++++++

In order to instantiate a contract, you need the minimum balance required for a Solana account of a given
size. There is a function ``minimum_balance(uint64 space)`` defined in
`minimum_balance.sol <https://github.com/hyperledger-solang/solang/blob/main/solana-library/minimum_balance.sol>`_
to calculate this.

.. _account_management:

Solana Account Management
_________________________

In a contract constructor, one can optionally write the ``@payer`` annotation, which receives a character sequence as
an argument. This annotation defines a Solana account that is going to pay for the initialization of the contract's data
account. The syntax ``@payer(my_account)`` declares an account named ``my_account``, which will be
required for every call to the constructor.

Similarly, for external functions in a contract, one can declare the necessary accounts, using function annotations.
``@account(myAcc)`` declares a read only account ``myAcc``, while ``@mutableAccount(otherAcc)`` declares a mutable
account ``otherAcc``. For signer accounts, the annotations follow the syntax ``@signer(mySigner)`` and
``@mutableSigner(myOtherSigner)``.


Accessing accounts' data
++++++++++++++++++++++++

Accounts declared on a constructor using the ``@payer`` annotation are available for access inside it. Likewise,
accounts declared on external functions with any of the aforementioned annotations are also available in the
``tx.accounts`` vector for easy access. For an account declared as ``@account(funder)``, the access follows the
syntax ``tx.accounts.funder``, which returns the :ref:`AccountInfo builtin struct <account_info>`.

.. include:: ../examples/solana/account_access.sol
  :code: solidity

.. _solana_cpi_accounts:

External calls with accounts
++++++++++++++++++++++++++++

In any Solana cross program invocation, including constructor calls, all the accounts a transaction needs must be
informed. If the ``{accounts: ...}`` call argument is missing from an external call, the compiler will automatically
generate the ``AccountMeta`` array that satisfies such a requirement. Currently, that only works if the call is done
in an function declared external, as shown in the example below. In any other case, the ``AccountMeta`` array must
be manually created, following the account ordering the IDL file specifies. If a certain call does not need any accounts,
an empty vector must be passed ``{accounts: []}``.

The following example shows two correct ways of calling a contract. Note that the IDL for the ``BeingBuilt`` contract
has an instruction called ``new``, representing the contract's constructor, whose accounts are specified in the
following order: ``dataAccount``, ``payer_account``, ``systemAccount``. That is the order one must follow when invoking
such a constructor.

.. include:: ../examples/solana/payer_annotation.sol
  :code: solidity

