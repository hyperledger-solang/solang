Solana
______

The Solana target requires `Solana <https://www.solana.com/>`_ v1.8.1.

Solana has the following differences to Ethereum Solidity:

- The address type is 32 bytes, not 20 bytes. This is what Solana calls an "account"
- An address literal has to be specified using the ``address"36VtvSbE6jVGGQytYWSaDPG7uZphaxEjpJHUUpuUbq4D"`` syntax
- There is no ``ecrecover()`` builtin function, but there is a ``signatureVerify()`` function which can check ed25519
  signatures.
- Solana has no concept of gas, so there is no gas functions
- Solana balance is stored in a ``uint64``, so ``msg.value``, *address* ``.balance``, ``.transfer()`` and ``.send()``
  all use uint64 rather than `uint256`.

This is how to build your Solidity for Solana:

.. code-block:: bash

  solang --target solana flipper.sol -v

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

        await contract.deploy('flipper', [true], program, storage, 17);

        const res = await contract.functions.get();
        console.log('state: ' + res.result);

        await contract.functions.flip();

        const res2 = await contract.functions.get();
        console.log('state: ' + res2.result);
    })();

The contract can be used via the `@solana/solidity <https://www.npmjs.com/package/@solana/solidity>`_  npm package. This
package has `documentation <https://solana-labs.github.io/solana-solidity.js/>`_ and there
are `some examples <https://solana-labs.github.io/solana-solidity.js/>`_. There is also
`solang's integration tests <https://github.com/hyperledger-labs/solang/tree/main/integration/solana>`_.
