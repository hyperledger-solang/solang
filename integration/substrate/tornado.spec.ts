import expect from 'expect';
import { weight, createConnection, deploy, transaction, aliceKeypair, daveKeypair, } from './index';
import { ContractPromise } from '@polkadot/api-contract';
import { ApiPromise } from '@polkadot/api';
import { DecodedEvent } from '@polkadot/api-contract/types';
import { KeyringPair } from '@polkadot/keyring/types';
import { createNote, init, parseNote, toHex, withdraw, } from './tornado/tornado'

const value = 1000000000000;
const merkle_tree_height = 20;

function addrToBigInt(uint8Array: Uint8Array): bigint {
    let result = BigInt(0);
    for (let i = 0; i < uint8Array.length; i++) {
        result <<= BigInt(8); // Left shift by 8 bits
        result += BigInt(uint8Array[i]); // Add the current byte
    }
    return result;
}

describe('Deploy tornado contracts and test them', () => {
    let conn: ApiPromise;
    let tornado: ContractPromise;
    let alice: KeyringPair;
    let dave: KeyringPair;
    let deposits: { noteString: string; commitment: string; }[];

    before(async function () {
        alice = aliceKeypair();
        dave = daveKeypair();

        // Deploy the ETHTornado contract
        conn = await createConnection();
        let hasher_contract = await deploy(conn, alice, 'Hasher.contract', 0n);
        let verifier_contract = await deploy(conn, alice, 'Verifier.contract', 0n);
        let parameters =
            [
                verifier_contract.address,
                hasher_contract.address,
                value,
                merkle_tree_height
            ];
        let tornado_contract = await deploy(conn, alice, 'ETHTornado.contract', 0n, ...parameters);
        tornado = new ContractPromise(conn, tornado_contract.abi, tornado_contract.address);

        // Create some deposit notes
        await init({});
        deposits = [createNote({}), createNote({})];

        // Deposit some funds to the tornado contract
        let gasLimit = await weight(conn, tornado, "deposit", [deposits[0].commitment], value);
        await transaction(tornado.tx.deposit({ gasLimit, value }, deposits[0].commitment), alice);

        gasLimit = await weight(conn, tornado, "deposit", [deposits[1].commitment], value);
        await transaction(tornado.tx.deposit({ gasLimit, value }, deposits[1].commitment), dave);
    });

    after(async function () {
        await conn.disconnect();
    });

    it('Withdraws funds from alice', async function () {
        this.timeout(50000);

        let proof = await withdraw(addrToBigInt(dave.addressRaw), deposits[0].noteString);
        let parameters = [
            proof.proof,
            proof.args[0], // Merkle root
            proof.args[1], // Nullifier hash
            proof.args[2], // Recipient address
        ];
        let gasLimit = await weight(conn, tornado, "withdraw", parameters);
        await transaction(tornado.tx.withdraw({ gasLimit }, ...parameters), alice);
    });
});