// SPDX-License-Identifier: Apache-2.0

import { loadContract } from "./setup";
import { Keypair, PublicKey } from "@solana/web3.js";
import { utils } from '@coral-xyz/anchor';
import expect from "expect";

describe('PDA hash table', function () {
    // A PDA (Program derived address) hash table is a way to store values for a provided key
    // on a unique account on chain, resembling a hash table. This is an example for achieving
    // so with Solidity.

    it('Table functions', async function test_table() {
        const { program, payer } = await loadContract("UserStats");
        // A user's public key will be the key for the hash table in this example.
        const myUser = Keypair.generate();

        // The actual 'hash' for our hash table is PDA. We utilize `findProgramAddress`, using the user's
        // public key as a seed and a 'user-stats' as another seed for randomness. This function will
        // return the same bump and PDA if the seeds and the program id are the same.
        const [userStatsPDA, bump] = PublicKey.findProgramAddressSync(
            [
                utils.bytes.utf8.encode('user-stats'),
                myUser.publicKey.toBuffer(),
            ],
            program.programId
        );

        // We create the account to hold the user's related information. The generated PDA becomes the
        // data account for our contract.
        // If a contract for `userStatsPDA` already exists, this function will fail.
        await program.methods.new(myUser.publicKey, bump, "user-one", 25)
            .accounts({
                dataAccount: userStatsPDA,
                wallet: payer.publicKey,
            })
            .signers([payer])
            .rpc();

        // To read the information from the contract, the data account is also necessary
        // If there is no contract created for `userStatsPDA`, this function will fail.
        let res = await program.methods.returnStats()
            .accounts({ dataAccount: userStatsPDA })
            .view();

        expect(res.return0).toBe("user-one");
        expect(res.return1).toBe(25);
        expect(res.return2).toBe(bump);

        // These function update the information in the contract.
        // If there is no contract created for `userStatsPDA`, these calls will fail.
        await program.methods.changeUserName("new-user-one")
            .accounts({ dataAccount: userStatsPDA })
            .rpc();
        await program.methods.changeLevel(20)
            .accounts({ dataAccount: userStatsPDA })
            .rpc();
        res = await program.methods.returnStats()
            .accounts({ dataAccount: userStatsPDA })
            .view();

        expect(res.return0).toBe("new-user-one");
        expect(res.return1).toBe(20);
        expect(res.return2).toBe(bump);
    });
});