import { Keypair } from "@stellar/stellar-base";
import { SignAuthEntry, SignTransaction } from "./types";
/**
 * For use with {@link Client} and {@link module:contract.AssembledTransaction}.
 * Implements `signTransaction` and `signAuthEntry` with signatures expected by
 * those classes. This is useful for testing and maybe some simple Node
 * applications. Feel free to use this as a starting point for your own
 * Wallet/TransactionSigner implementation.
 *
 * @memberof module:contract
 *
 * @param {Keypair} keypair {@link Keypair} to use to sign the transaction or auth entry
 * @param {string} networkPassphrase passphrase of network to sign for
 */
export declare const basicNodeSigner: (keypair: Keypair, networkPassphrase: string) => {
    signTransaction: SignTransaction;
    signAuthEntry: SignAuthEntry;
};
