// Tests against the tornado cash core contracts.
// The tornado contracts used here contain minor mechanical changes to work fine on Polkadot.
// The ZK-SNARK setup is the same as ETH Tornado on mainnet.
// On the node, the MiMC sponge hash (available as EVM bytecode) and bn128 curve operations
// (precompiled contracts on Ethereum) are expected to be implemented as chain extensions.

import expect from 'expect';
import { weight, createConnection, deploy, transaction, aliceKeypair, daveKeypair, debug_buffer, } from './index';
import { ContractPromise } from '@polkadot/api-contract';
import { ApiPromise } from '@polkadot/api';
import { KeyringPair } from '@polkadot/keyring/types';
import { createNote, init_snark, toHex, withdraw, } from './tornado/tornado'

type Deposit = { noteString: string; commitment: string; };

let deposits: Deposit[];
const denomination = 1000000000000n;
const merkle_tree_height = 20;

function addressToBigInt(uint8Array: Uint8Array): bigint {
    let result = BigInt(0);
    for (let i = 0; i < uint8Array.length; i++) {
        result <<= BigInt(8); // Left shift by 8 bits
        result += BigInt(uint8Array[i]); // Add the current byte
    }
    return result;
}

// Generate a ZK proof needed to withdraw funds. Uses the deposit at the given `index`. 
async function generateProof(recipient: KeyringPair, index: number): Promise<string[]> {
    const to = addressToBigInt(recipient.addressRaw);
    // In production, we'd fetch and parse all events, which is too cumbersome for this PoC.
    const leaves = deposits.map(e => e.commitment);
    const proof = await withdraw(to, deposits[index].noteString, leaves);
    return [
        proof.proof,
        proof.args[0],  // Merkle root
        proof.args[1],  // Nullifier hash
        toHex(to),      // The contract will mod it over the finite field
    ];
}

describe('Deploy the tornado contract, create 2 deposits and withdraw them afterwards', () => {
    let conn: ApiPromise;
    let tornado: ContractPromise;
    let alice: KeyringPair;
    let dave: KeyringPair;

    before(async function () {
        alice = aliceKeypair();
        dave = daveKeypair();

        // Deploy hasher, verifier and tornado contracts
        conn = await createConnection();
        const hasher_contract = await deploy(conn, alice, 'Hasher.contract', 0n);
        const verifier_contract = await deploy(conn, alice, 'Verifier.contract', 0n);
        const parameters =
            [
                verifier_contract.address,
                hasher_contract.address,
                denomination,
                merkle_tree_height
            ];
        const tornado_contract = await deploy(conn, alice, 'NativeTornado.contract', 0n, ...parameters);
        tornado = new ContractPromise(conn, tornado_contract.abi, tornado_contract.address);

        // Deposit some funds to the tornado contract
        await init_snark({});
        deposits = [createNote({}), createNote({})];

        let gasLimit = await weight(conn, tornado, 'deposit', [deposits[0].commitment], denomination);
        let tx = tornado.tx.deposit({ gasLimit, value: denomination }, deposits[0].commitment);
        await transaction(tx, alice);

        gasLimit = await weight(conn, tornado, 'deposit', [deposits[1].commitment], denomination);
        tx = tornado.tx.deposit({ gasLimit, value: denomination }, deposits[1].commitment);
        await transaction(tx, dave);
    });

    after(async function () {
        await conn.disconnect();
    });

    it('Withdraws funds deposited by alice to dave', async function () {
        this.timeout(50000);

        const { data: { free: balanceBefore } } = await conn.query.system.account(dave.address);

        const parameters = await generateProof(dave, 0);
        const gasLimit = await weight(conn, tornado, 'withdraw', parameters);
        await transaction(tornado.tx.withdraw({ gasLimit }, ...parameters), alice);

        expect(balanceBefore.toBigInt() + denomination)
            .toEqual((await conn.query.system.account(dave.address)).data.free.toBigInt());

        expect(await debug_buffer(conn, tornado, 'withdraw', parameters))
            .toContain('The note has been already spent');
    });

    it('Withdraws funds deposited by dave to alice', async function () {
        this.timeout(50000);

        const { data: { free: balanceBefore } } = await conn.query.system.account(dave.address);

        const parameters = await generateProof(dave, 1);
        const gasLimit = await weight(conn, tornado, 'withdraw', parameters);
        await transaction(tornado.tx.withdraw({ gasLimit }, ...parameters), alice);

        expect(balanceBefore.toBigInt() + denomination)
            .toEqual((await conn.query.system.account(dave.address)).data.free.toBigInt());

        expect(await debug_buffer(conn, tornado, 'withdraw', parameters))
            .toContain('The note has been already spent');
    });

    it('Fails to withdraw without a valid proof', async function () {
        this.timeout(50000);

        // Without a corresponding deposit, this merkle root should not exist yet
        deposits.push(createNote({}));
        let parameters = await generateProof(alice, 2);
        expect(await debug_buffer(conn, tornado, 'withdraw', parameters))
            .toContain('Cannot find your merkle root');

        const gasLimit = await weight(conn, tornado, 'deposit', [deposits[2].commitment], denomination);
        const tx = tornado.tx.deposit({ gasLimit, value: denomination }, deposits[2].commitment);
        await transaction(tx, alice);

        // Messing up the proof should result in a curve pairing failure
        parameters[0] = parameters[0].substring(0, parameters[0].length - 4) + "0000";
        expect(await debug_buffer(conn, tornado, 'withdraw', parameters))
            .toContain('pairing-opcode-failed');
    });
});
