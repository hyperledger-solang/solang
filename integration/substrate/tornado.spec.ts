import expect from 'expect';
import { weight, createConnection, deploy, transaction, aliceKeypair, daveKeypair, } from './index';
import { ContractPromise } from '@polkadot/api-contract';
import { ApiPromise } from '@polkadot/api';
import { DecodedEvent } from '@polkadot/api-contract/types';
import { KeyringPair } from '@polkadot/keyring/types';
import { createNote, init, parseNote, } from './tornado/tornado'

const value = 1000000000000;
const merkle_tree_height = 20;

describe('Deploy tornado contracts and test them', () => {
    let conn: ApiPromise;
    let tornado: ContractPromise;
    let alice: KeyringPair;
    let dave: KeyringPair;
    let deposits;

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

    it('Withdraws the funds from alice', async function () {
        this.timeout(50000);

    });
});
