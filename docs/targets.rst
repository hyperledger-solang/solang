Target Specific
===============

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
        while (true) {
            console.log('Airdropping SOL to a new wallet ...');
            await connection.requestAirdrop(payer.publicKey, 1 * LAMPORTS_PER_SOL);
            await new Promise((resolve) => setTimeout(resolve, 1000));
            if (await connection.getBalance(payer.publicKey)) break;
        }

        const program = await Program.load(connection, payer, Keypair.generate(), PROGRAM_SO);

        console.log('Program deployment finished, deploying the flipper contract ...');

        const storageKeyPair = Keypair.generate();
        const deployRes = await program.deployContract({
            name: "flipper",
            abi: FLIPPER_ABI,
            storageKeyPair,
            constructorArgs: [true],
            space: 17,
        });

        const contract = deployRes.contract;

        const res = await contract.functions.get({ simulate: true });
        console.log('state: ' + res.result);

        await contract.functions.flip();

        const res2 = await contract.functions.get({ simulate: true });
        console.log('state: ' + res2.result);

        process.exit(0);
    })();

The contract can be used via the `@solana/solidity <https://www.npmjs.com/package/@solana/solidity>`_  npm package. This
package has `documentation <https://solana-labs.github.io/solana-solidity.js/>`_ and there
are `some examples <https://solana-labs.github.io/solana-solidity.js/>`_. There is also
`solang's integration tests <https://github.com/hyperledger-labs/solang/tree/main/integration/solana>`_.

Parity Substrate
________________

Solang works with Parity Substrate 2.0 or later.

The Parity Substrate has the following differences to Ethereum Solidity:

- The address type is 32 bytes, not 20 bytes. This is what Substrate calls an "account"
- An address literal has to be specified using the ``address"5GBWmgdFAMqm8ZgAHGobqDqX6tjLxJhv53ygjNtaaAn3sjeZ"`` syntax
- ABI encoding and decoding is done using the `SCALE <https://substrate.dev/docs/en/knowledgebase/advanced/codec>`_ encoding
- Multiple constructors are allowed, and can be overloaded
- There is no ``ecrecover()`` builtin function, or any other function to recover or verify cryptographic signatures at runtime
- Only functions called via rpc may return values; when calling a function in a transaction, the return values cannot be accessed
- An `assert()`, `require()`, or `revert()` executes the wasm unreachable instruction. The reason code is lost

There is an solidity example which can be found in the
`examples <https://github.com/hyperledger-labs/solang/tree/main/examples>`_
directory. Write this to flipper.sol and run:

.. code-block:: bash

  solang --target substrate flipper.sol

Now you should have a file called ``flipper.contract``. The file contains both the ABI and contract wasm.
It can be used directly in the
`Polkadot UI <https://substrate.dev/substrate-contracts-workshop/#/0/deploy-contract>`_, as if the contract was written in ink!.


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
