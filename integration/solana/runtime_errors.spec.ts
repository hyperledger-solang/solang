// SPDX-License-Identifier: Apache-2.0

import { Keypair, PublicKey, sendAndConfirmTransaction, SystemProgram, Transaction } from '@solana/web3.js';
import expect from 'expect';
import { loadContractAndCallConstructor } from './setup';
import { Program, Provider, BN, AnchorProvider } from '@coral-xyz/anchor';
import { createAccount } from "@solana/spl-token";

describe('Runtime Errors', function () {
    this.timeout(150000);

    let program: Program;
    let storage: Keypair;
    let payer: Keypair;
    let provider: Provider;

    before(async function () {
        ({ program, storage, payer, provider } = await loadContractAndCallConstructor('RuntimeErrors'));
    });

    it('Prints runtime errors', async function () {

        try {
            let res = await program.methods.setStorageBytes().accounts({ dataAccount: storage.publicKey }).simulate();
        }
        catch (e: any) {
            const logs = e.simulationResponse.logs;
            expect(logs).toContain(`Program log: runtime_error: storage index out of bounds in runtime_errors.sol:41:11-12,
`);
        }

        try {
            let res = await program.methods.getStorageBytes().accounts({ dataAccount: storage.publicKey }).simulate();;
        }
        catch (e: any) {
            const logs = e.simulationResponse.logs;
            expect(logs).toContain(`Program log: runtime_error: storage array index out of bounds in runtime_errors.sol:48:19-23,
`);
        }

        try {
            let res = await program.methods.popEmptyStorage().accounts({ dataAccount: storage.publicKey }).simulate();
        } catch (e: any) {
            const logs = e.simulationResponse.logs;
            expect(logs).toContain(`Program log: runtime_error: pop from empty storage array in runtime_errors.sol:60:9-12,
`)

        }

        try {
            let res = await program.methods.invalidInstruction().simulate();
        } catch (e: any) {
            const logs = e.simulationResponse.logs;
            expect(logs).toContain(`Program log: runtime_error: reached invalid instruction in runtime_errors.sol:101:13-22,
`)

        }

        try {
            let res = await program.methods.byteCastFailure(new BN(33)).simulate();
        } catch (e: any) {
            const logs = e.simulationResponse.logs;
            expect(logs).toContain(`Program log: runtime_error: bytes cast error in runtime_errors.sol:107:23-40,
`)

        }

        try {
            let res = await program.methods.iWillRevert().simulate();
        } catch (e: any) {
            const logs = e.simulationResponse.logs;
            expect(logs).toContain(`Program log: runtime_error: revert encountered in runtime_errors.sol:69:9-17,
`)
        }

        try {
            let res = await program.methods.assertTest(new BN(9)).simulate();
        } catch (e: any) {
            const logs = e.simulationResponse.logs;
            expect(logs).toContain(`Program log: runtime_error: assert failure in runtime_errors.sol:34:16-24,
`)
        }

        try {
            let res = await program.methods.writeIntegerFailure(new BN(1)).simulate();
        } catch (e: any) {
            const logs = e.simulationResponse.logs;
            expect(logs).toContain(`Program log: runtime_error: integer too large to write in buffer in runtime_errors.sol:74:18-31,
`)
        }

        try {
            let res = await program.methods.writeBytesFailure(new BN(9)).simulate();
        } catch (e: any) {
            const logs = e.simulationResponse.logs;
            expect(logs).toContain(`Program log: runtime_error: data does not fit into buffer in runtime_errors.sol:80:18-28,
`)
        }


        try {
            let res = await program.methods.readIntegerFailure(new BN(2)).simulate();
        } catch (e: any) {
            const logs = e.simulationResponse.logs;
            expect(logs).toContain(`Program log: runtime_error: read integer out of bounds in runtime_errors.sol:85:18-30,
`)
        }


        try {
            let res = await program.methods.outOfBounds(new BN(19)).simulate();
        } catch (e: any) {
            const logs = e.simulationResponse.logs;
            expect(logs).toContain(`Program log: runtime_error: array index out of bounds in runtime_errors.sol:96:16-21,
`)
        }


        try {
            let res = await program.methods.truncFailure(new BN(99999999999999)).simulate();
        } catch (e: any) {
            const logs = e.simulationResponse.logs;
            expect(logs).toContain(`Program log: runtime_error: truncated type overflows in runtime_errors.sol:90:37-42,
`)
        }

    });

});

