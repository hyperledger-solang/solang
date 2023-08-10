// SPDX-License-Identifier: Apache-2.0


import {loadContractAndCallConstructor, newConnectionAndPayer} from "./setup";
import {Connection, Keypair, PublicKey} from "@solana/web3.js";
import {
    approveChecked,
    AuthorityType,
    createMint,
    getAccount,
    getOrCreateAssociatedTokenAccount, mintTo,
    setAuthority
} from "@solana/spl-token";
import { publicKey, u64, bool } from '@solana/buffer-layout-utils';
import { u32, u8, struct } from '@solana/buffer-layout';
import expect from "expect";
import {Program} from "@coral-xyz/anchor";
import assert from "assert";
import exp from "constants";

describe('Deserialize account data', function () {
    this.timeout(500000);

    interface RawMint {
        mintAuthorityOption: 1 | 0;
        mintAuthority: PublicKey;
        supply: bigint;
        decimals: number;
        isInitialized: boolean;
        freezeAuthorityOption: 1 | 0;
        freezeAuthority: PublicKey;
    }

    const MintLayout = struct<RawMint>([
        u32('mintAuthorityOption'),
        publicKey('mintAuthority'),
        u64('supply'),
        u8('decimals'),
        bool('isInitialized'),
        u32('freezeAuthorityOption'),
        publicKey('freezeAuthority'),
    ]);

    let program: Program;
    let storage: Keypair;
    let connection: Connection;
    let payer: Keypair;

    before(async function (){
        ({ program, storage } = await loadContractAndCallConstructor('AccountData'));
        ([connection, payer] = newConnectionAndPayer()) ;
    });

    it('token account', async function check_token_account() {
        const mint_authority = Keypair.generate();
        const freeze_authority = Keypair.generate();

        const mint = await createMint(
            connection,
            payer,
            mint_authority.publicKey,
            freeze_authority.publicKey,
            0
        );

        const owner = Keypair.generate();

        let token_account = await getOrCreateAssociatedTokenAccount(
            connection,
            payer,
            mint,
            owner.publicKey
        );

        let res = await program.methods.tokenAccount(token_account.address)
            .accounts({dataAccount: storage.publicKey})
            .remainingAccounts(
                [
                    {pubkey: token_account.address, isSigner: false, isWritable: false}
                ]
            )
            .view();

        expect(res.mintAccount).toEqual(token_account.mint);
        expect(res.owner).toEqual(token_account.owner);
        expect(res.balance.toString()).toEqual(token_account.amount.toString());
        expect(res.delegatePresent).toEqual(token_account.delegate != null);
        expect(res.delegate).toEqual(new PublicKey("11111111111111111111111111111111")); // 0
        expect(res.state).toEqual({"initialized": {}});
        expect(res.isNativePresent).toEqual(token_account.rentExemptReserve != null);
        expect(res.isNative.toString()).toEqual("0");
        expect(res.delegatedAmount.toString()).toEqual(token_account.delegatedAmount.toString());
        expect(res.closeAuthorityPresent).toEqual(token_account.closeAuthority != null);
        expect(res.closeAuthority).toEqual(new PublicKey("11111111111111111111111111111111")); // 0

        const delegate_account = Keypair.generate();
        // delegate tokens
        await approveChecked(
            connection,
            payer,
            mint,
            token_account.address,
            delegate_account.publicKey,
            owner,
            1,
            0
        );
        token_account = await getAccount(connection, token_account.address);

        res = await program.methods.tokenAccount(token_account.address)
            .accounts({dataAccount: storage.publicKey})
            .remainingAccounts(
                [
                    {pubkey: token_account.address, isSigner: false, isWritable: false}
                ]
            )
            .view();

        // The delegate account should be present now
        expect(res.delegatePresent).toEqual(token_account.delegate !=  null);
        expect(res.delegate).toEqual(token_account.delegate);

        const close_authority = Keypair.generate();
        // close authority
        await setAuthority(
            connection,
            payer,
            token_account.address,
            owner,
            AuthorityType.CloseAccount,
            close_authority.publicKey
        );
        token_account = await getAccount(connection, token_account.address);

        res = await program.methods.tokenAccount(token_account.address)
            .accounts({dataAccount: storage.publicKey})
            .remainingAccounts(
                [
                    {pubkey: token_account.address, isSigner: false, isWritable: false}
                ]
            )
            .view();

        // The close authority should be present
        expect(res.closeAuthorityPresent).toEqual(token_account.closeAuthority != null);
        expect(res.closeAuthority).toEqual(close_authority.publicKey);

        const sol_mint = new PublicKey("So11111111111111111111111111111111111111112");
        token_account = await getOrCreateAssociatedTokenAccount(
            connection,
            payer,
            sol_mint,
            owner.publicKey
        );

        res = await program.methods.tokenAccount(token_account.address)
            .accounts({dataAccount: storage.publicKey})
            .remainingAccounts(
                [
                    {pubkey: token_account.address, isSigner: false, isWritable: false}
                ]
            )
            .view();

        // Is native must be present
        expect(res.isNativePresent).toEqual(token_account.isNative);
        expect(res.isNativePresent).toEqual(token_account.rentExemptReserve != null);
        expect(res.isNative.toString()).toEqual(token_account.rentExemptReserve!.toString());
    });

    it('mint account', async function mint_account() {
        const mint_authority = Keypair.generate();
        const freeze_authority = Keypair.generate();

        const mint = await createMint(
            connection,
            payer,
            mint_authority.publicKey,
            freeze_authority.publicKey,
            2
        );

        const owner = Keypair.generate();

        const token_account = await getOrCreateAssociatedTokenAccount(
            connection,
            payer,
            mint,
            owner.publicKey
        );

        await mintTo(
            connection,
            payer,
            mint,
            token_account.address,
            mint_authority,
            5
        );

        let data = await connection.getAccountInfo(mint);
        assert(data != null);
        let decoded = MintLayout.decode(data.data);

        let res = await program.methods.mintAccount(mint)
            .accounts({dataAccount: storage.publicKey})
            .remainingAccounts(
                [
                    {pubkey: mint, isWritable: false, isSigner: false}
                ]
            )
            .view();

        // Authorities are present
        expect(res.authorityPresent).toEqual(decoded.mintAuthorityOption > 0);
        expect(res.mintAuthority).toEqual(decoded.mintAuthority);
        expect(res.supply.toString()).toEqual(decoded.supply.toString())
        expect(res.decimals).toEqual(decoded.decimals);
        expect(res.isInitialized).toEqual(decoded.isInitialized);
        expect(res.freezeAuthorityPresent).toEqual(decoded.freezeAuthorityOption > 0);
        expect(res.freezeAuthority).toEqual(decoded.freezeAuthority);

        await setAuthority(
            connection,
            payer,
            mint,
            mint_authority,
            AuthorityType.MintTokens,
            null
        );

        await setAuthority(
            connection,
            payer,
            mint,
            freeze_authority,
            AuthorityType.FreezeAccount,
            null
        );

        data = await connection.getAccountInfo(mint);
        assert(data != null);
        decoded = MintLayout.decode(data.data);

        res = await program.methods.mintAccount(mint)
            .accounts({dataAccount: storage.publicKey})
            .remainingAccounts(
                [
                    {pubkey: mint, isWritable: false, isSigner: false}
                ]
            )
            .view();

        // Authorities are not present
        expect(res.authorityPresent).toEqual(decoded.mintAuthorityOption > 0);
        expect(res.supply.toString()).toEqual(decoded.supply.toString())
        expect(res.decimals).toEqual(decoded.decimals);
        expect(res.isInitialized).toEqual(decoded.isInitialized);
        expect(res.freezeAuthorityPresent).toEqual(decoded.freezeAuthorityOption > 0);
    });
});