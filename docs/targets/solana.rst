Solana
======

The Solana target requires `Solana <https://www.solana.com/>`_ v1.8.1.

Solana has the following differences to Ethereum Solidity:

- The address type is 32 bytes, not 20 bytes. This is what Solana calls an "account"
- An address literal has to be specified using the ``address"36VtvSbE6jVGGQytYWSaDPG7uZphaxEjpJHUUpuUbq4D"`` syntax
- There is no ``ecrecover()`` builtin function, but there is a ``signatureVerify()`` function which can check ed25519
  signatures.
- Solana has no concept of gas, so there is no gas functions
- Solana balance is stored in a ``uint64``, so *address* ``.balance``, ``.transfer()`` and ``.send()``
  all use ``uint64`` rather than ``uint256``.

This is how to build your Solidity for Solana:

.. code-block:: bash

  solang compile --target solana flipper.sol -v

This will produce two files called `flipper.abi` and `bundle.so`. The first is an ethereum style abi file and the latter being
the ELF BPF shared object which can be deployed on Solana. For each contract, an abi file will be created; a single `bundle.so`
is created which contains the code all the contracts provided on the command line.

.. code-block:: bash

    npm install @solana/solidity

Now run the following javascript by saving it to `flipper.js` and running it with ``node flipper.js``.

.. code-block:: javascript

    const { Connection, LAMPORTS_PER_SOL, Keypair } = require('@solana/web3.js');
    const { Contract, Program } = require('@solana/solidity');
    const { readFileSync } = require('fs');

    const FLIPPER_ABI = JSON.parse(readFileSync('./flipper.abi', 'utf8'));
    const PROGRAM_SO = readFileSync('./bundle.so');

    (async function () {
        console.log('Connecting to your local Solana node ...');
        const connection = new Connection('http://localhost:8899', 'confirmed');

        const payer = Keypair.generate();

        console.log('Airdropping SOL to a new wallet ...');
        const signature = await connection.requestAirdrop(payer.publicKey, LAMPORTS_PER_SOL);
        await connection.confirmTransaction(signature, 'confirmed');

        const program = Keypair.generate();
        const storage = Keypair.generate();

        const contract = new Contract(connection, program.publicKey, storage.publicKey, FLIPPER_ABI, payer);

        await contract.load(program, PROGRAM_SO);

        console.log('Program deployment finished, deploying the flipper contract ...');

        await contract.deploy('flipper', [true], storage, 17);

        const res = await contract.functions.get();
        console.log('state: ' + res.result);

        await contract.functions.flip();

        const res2 = await contract.functions.get();
        console.log('state: ' + res2.result);
    })();

The contract can be used via the `@solana/solidity <https://www.npmjs.com/package/@solana/solidity>`_  npm package. This
package has `documentation <https://solana-labs.github.io/solana-solidity.js/>`_ and there
are `some examples <https://solana-labs.github.io/solana-solidity.js/>`_. There is also
`solang's integration tests <https://github.com/hyperledger/solang/tree/main/integration/solana>`_.

.. _call_anchor:

Calling Anchor Programs from Solidity
_____________________________________

It is possible to call `Anchor Programs <https://github.com/coral-xyz/anchor>`_
from Solidity. You first have to generate an interface file from the IDL file using
the :ref:`idl_command`. Then, import the Solidity file in your Solidity using the
``import "...";`` syntax. Say you have an anchor program called `solidity_from_idl` with a
function `idl_defined_function`, you can call it like so:

.. code-block:: solidity

    import "solidity_from_idl.sol";
    import "solana";

    contract example {
        function test(addres a, address b) public {
            // The list of accounts to pass into the Anchor program must be passed
            // as an array of AccountMeta with the correct writable/signer flags set
            AccountMeta[2] am = [
                AccountMeta({pubkey: a, is_writable: true, is_signer: false}),
                AccountMeta({pubkey: b, is_writable: false, is_signer: false})
            ];

            // Any return values are decoded automatically
            int64 res = solidity_from_idl.idl_defined_function{accounts: am}(arg1, arg2);
        }
    }

.. _value_transfer:

Transfering native value with a function call
_____________________________________________

The Solidity langauge on Ethereum allows value transfers with an external call
or constructor, using the ``auction.bid{value: 501}()`` syntax.
Solana Cross Program Invocation (CPI) does not support this. This means that:

 - Specifying ``value:`` on an external call or constructor is not permitted
 - The ``payable`` keyword has no effect
 - ``msg.value`` is not supported

.. note::

    Hypothetically, this could be implemented like so: the caller could transfer
    native balance and then inform the callee about the amount transferred by
    specifying this in the instruction data. However, it would be trivial to
    forge such an operation.

Receive function
________________

In Solidity the ``receive()`` function, when defined, is called whenever the native
balance for an account gets credited, for example through a contract calling
``account.transfer(value);``. On Solana, there is no method that implement
this. The balance of an account can be credited without any code being executed.

``receive()`` functions are not permitted on the Solana target.

Builtin Imports
________________

Some builtin functionality is only available after importing. The following structs
can be imported via the special import file ``solana``.

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

bytes ``data```
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
`program derived adddresses <https://edge.docs.solana.com/developing/programming-model/calling-between-programs#program-derived-addresses>`_.

.. code-block:: solidity

    import {create_program_address} from 'solana';

    contract pda {
        address token = address"TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

        function create_pda(bytes seed2) public returns (address) {
            return create_program_address(["kabang", seed2], token);
        }
    }

Builtin try_find_program_address
++++++++++++++++++++++++++++++++

This function returns the program derived address for a program address and
the provided seeds, along with a seed bump. See the Solana documentation on
`program derived adddresses <https://edge.docs.solana.com/developing/programming-model/calling-between-programs#program-derived-addresses>`_.

.. code-block:: solidity

    import {try_find_program_address} from 'solana';

    contract pda {
        address token = address"TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

        function create_pda(bytes seed2) public returns (address, bytes1) {
            return try_find_program_address(["kabang", seed2], token);
        }
    }



Solana Library
______________

In Solang's Github repository, there is a directory called ``solana-library``. It contains libraries for Solidity contracts
to interact with Solana specific instructions. Currently, there are two libraries there: one for SPL tokens and another
for Solana's system instructions. In order to use those functionalities, copy the correspondent library
file to your project and import it.

SPL-token
+++++++++

`spl-token <https://spl.solana.com/token>`_ is the Solana native way of creating tokens, minting, burning and
transferring token. This is the Solana equivalent of
`ERC-20 <https://ethereum.org/en/developers/docs/standards/tokens/erc-20/>`_ and
`ERC-721 <https://ethereum.org/en/developers/docs/standards/tokens/erc-721/>`_. Solang's repository contains
a library ``SplToken`` to use spl-token from Solidity. The file
`spl_token.sol <https://github.com/hyperledger/solang/blob/main/solana-library/spl_token.sol>`_  should be copied into
your source tree, and then imported in your solidity files where it is required. The ``SplToken`` library has doc
comments explaining how it should be used.

There is an example in our integration tests of how this should be used. See
`token.sol <https://github.com/hyperledger/solang/blob/main/integration/solana/token.sol>`_ and
`token.spec.ts <https://github.com/hyperledger/solang/blob/main/integration/solana/token.spec.ts>`_.

System Instructions
+++++++++++++++++++

Solana's system instructions enables developers to interact with Solana's System Program. There are functions to
create new accounts, allocate account data, assign accounts to owning programs, transfer lamports from System Program
owned accounts and pay transaction fees. More information about the functions offered can be found both on
`Solana documentation <https://docs.rs/solana-program/1.11.10/solana_program/system_instruction/enum.SystemInstruction.html>`_
and on Solang's `system_instruction.sol <https://github.com/hyperledger/solang/blob/main/solana-library/system_instruction.sol>`_ file.

The usage of system instructions needs the correct setting of writable and signer accounts when interacting with Solidity
contracts on chain. Examples are available on Solang's integration tests.
See `system_instruction_example.sol <https://github.com/hyperledger/solang/blob/main/integration/solana/system_instruction_example.sol>`_
and `system_instruction.spec.ts <https://github.com/hyperledger/solang/blob/main/integration/solana/system_instruction.spec.ts>`_
