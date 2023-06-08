import expect from 'expect';
import { weight, createConnection, deploy, transaction, aliceKeypair, daveKeypair, } from './index';
import { ContractPromise } from '@polkadot/api-contract';
import { ApiPromise } from '@polkadot/api';
import { DecodedEvent } from '@polkadot/api-contract/types';
import { KeyringPair } from '@polkadot/keyring/types';
import { createNote, init, parseNote } from './tornado/tornado'

const value = 1000000000000;
const merkle_tree_height = 20;

describe('Deploy tornado contracts and test them', () => {
    let conn: ApiPromise;
    let tornado: ContractPromise;
    let alice: KeyringPair;
    let dave: KeyringPair;
    let deposits: [BigInt];

    before(async function () {
        alice = aliceKeypair();
        dave = daveKeypair();

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

        await init({});
    });

    after(async function () {
        await conn.disconnect();
    });

    it('deposit some funds to the contract', async function () {
        this.timeout(50000);

        let note = createNote({});
        let gasLimit = await weight(conn, tornado, "deposit", [BigInt(note.commitment)], value);
        let tx = tornado.tx.deposit({ gasLimit, value }, BigInt(note.commitment));
        let res0: any = await transaction(tx, alice);
        let events: DecodedEvent[] = res0.contractEvents;
        console.log(events[0]);
    });
});
