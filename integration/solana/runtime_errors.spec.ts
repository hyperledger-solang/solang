// SPDX-License-Identifier: Apache-2.0

import { Keypair, PublicKey } from '@solana/web3.js';
import expect from 'expect';
import { loadContract } from './setup';
import { Program, Provider, BN } from '@project-serum/anchor';

describe('Runtime Errors', function () {
    this.timeout(150000);

    let program: Program;
    let storage: Keypair;
    let payer: Keypair;
    let provider: Provider;

    before(async function () {
        ({ program, storage, payer, provider } = await loadContract('RuntimeErrors'));
    });

    it('Prints runtime errors', async function () {

        try {
            let res = await program.methods.setStorageBytes().accounts({ dataAccount: storage.publicKey }).simulate();
        }
        catch (e: any) {
            const logs = e.simulationResponse.logs;
            expect(logs).toContain("Program log: storage index out of bounds in runtime_errors.sol:42:10");
        }

        try {
            let res = await program.methods.getStorageBytes().accounts({ dataAccount: storage.publicKey }).simulate();;
        }
        catch (e: any) {
            const logs = e.simulationResponse.logs;
            expect(logs).toContain("Program log: storage array index out of bounds in runtime_errors.sol:49:18");
        }

        try {
            let res = await program.methods.popEmptyStorage().accounts({ dataAccount: storage.publicKey }).simulate();
        } catch (e: any) {
            const logs = e.simulationResponse.logs;
            expect(logs).toContain("Program log: pop from empty storage array in runtime_errors.sol:61:8")

        }

        try {
            let res = await program.methods.invalidInstruction().accounts({ dataAccount: storage.publicKey }).simulate();
        } catch (e: any) {
            const logs = e.simulationResponse.logs;
            expect(logs).toContain("Program log: reached invalid instruction in runtime_errors.sol:108:12")

        }

        try {
            let res = await program.methods.byteCastFailure(new BN(33)).accounts({ dataAccount: storage.publicKey }).simulate();
        } catch (e: any) {
            const logs = e.simulationResponse.logs;
            expect(logs).toContain("Program log: bytes cast error in runtime_errors.sol:114:22")

        }

        try {
            let res = await program.methods.iWillRevert().accounts({ dataAccount: storage.publicKey }).simulate();
        } catch (e: any) {
            const logs = e.simulationResponse.logs;
            expect(logs).toContain("Program log: revert encountered in runtime_errors.sol:76:8")
        }

        try {
            let res = await program.methods.assertTest(new BN(9)).accounts({ dataAccount: storage.publicKey }).simulate();
        } catch (e: any) {
            const logs = e.simulationResponse.logs;
            expect(logs).toContain("Program log: assert failure in runtime_errors.sol:35:15")
        }

        try {
            let res = await program.methods.writeIntegerFailure(new BN(1)).accounts({ dataAccount: storage.publicKey }).simulate();
        } catch (e: any) {
            const logs = e.simulationResponse.logs;
            expect(logs).toContain("Program log: integer too large to write in buffer in runtime_errors.sol:81:17")
        }

        try {
            let res = await program.methods.writeBytesFailure(new BN(9)).accounts({ dataAccount: storage.publicKey }).simulate();
        } catch (e: any) {
            const logs = e.simulationResponse.logs;
            expect(logs).toContain("Program log: data does not fit into buffer in runtime_errors.sol:87:17")
        }


        try {
            let res = await program.methods.readIntegerFailure(new BN(2)).accounts({ dataAccount: storage.publicKey }).simulate();
        } catch (e: any) {
            const logs = e.simulationResponse.logs;
            expect(logs).toContain("Program log: read integer out of bounds in runtime_errors.sol:92:17")
        }


        try {
            let res = await program.methods.outOfBounds(new BN(19)).accounts({ dataAccount: storage.publicKey }).simulate();
        } catch (e: any) {
            const logs = e.simulationResponse.logs;
            expect(logs).toContain("Program log: array index out of bounds in runtime_errors.sol:103:15")
        }


        try {
            let res = await program.methods.truncFailure(new BN(99999999999999)).accounts({ dataAccount: storage.publicKey }).simulate();
        } catch (e: any) {
            const logs = e.simulationResponse.logs;
            expect(logs).toContain("Program log: truncated type overflows in runtime_errors.sol:97:36")
        }

        let child_program = new PublicKey("Cre7AzxtwSxXwU2jekYtCAQ57DkBhY9SjGDLdcrwhAo6");
        let child = Keypair.generate();


        const signature = await program.methods.createChild(child.publicKey, payer.publicKey)
            .accounts({ dataAccount: storage.publicKey })
            .remainingAccounts([
                { pubkey: child_program, isSigner: false, isWritable: false },
                { pubkey: child.publicKey, isSigner: true, isWritable: true },
                { pubkey: payer.publicKey, isSigner: true, isWritable: true },
            ])
            .signers([payer, child])
            .rpc({ commitment: 'confirmed' });


        const tx = await provider.connection.getTransaction(signature, { commitment: 'confirmed' });
        try {
            const signature = await program.methods.createChild(child.publicKey, payer.publicKey)
                .accounts({ dataAccount: storage.publicKey })
                .remainingAccounts([
                    { pubkey: child_program, isSigner: false, isWritable: false },
                    { pubkey: payer.publicKey, isSigner: true, isWritable: true },
                ])
                .signers([payer]).simulate();
        } catch (e: any) {
            const logs = e.simulationResponse.logs
            expect(logs).toContain("Program log: contract creation failed in runtime_errors.sol:71:12")
        }

    });

});

