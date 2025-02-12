import { xdr, Account } from "@stellar/stellar-base";
import { Server } from "../rpc";
import { AssembledTransactionOptions } from "./types";
/**
 * Keep calling a `fn` for `timeoutInSeconds` seconds, if `keepWaitingIf` is
 * true. Returns an array of all attempts to call the function.
 * @private
 */
export declare function withExponentialBackoff<T>(
/** Function to call repeatedly */
fn: (previousFailure?: T) => Promise<T>, 
/** Condition to check when deciding whether or not to call `fn` again */
keepWaitingIf: (result: T) => boolean, 
/** How long to wait between the first and second call */
timeoutInSeconds: number, 
/** What to multiply `timeoutInSeconds` by, each subsequent attempt */
exponentialFactor?: number, 
/** Whether to log extra info */
verbose?: boolean): Promise<T[]>;
/**
 * If contracts are implemented using the `#[contracterror]` macro, then the
 * errors get included in the on-chain XDR that also describes your contract's
 * methods. Each error will have a specific number. This Regular Expression
 * matches these "expected error types" that a contract may throw, and helps
 * {@link AssembledTransaction} parse these errors.
 *
 * @constant {RegExp}
 * @default "/Error\(Contract, #(\d+)\)/"
 * @memberof module:contract.Client
 */
export declare const contractErrorPattern: RegExp;
/**
 * A TypeScript type guard that checks if an object has a `toString` method.
 * @private
 */
export declare function implementsToString(
/** some object that may or may not have a `toString` method */
obj: unknown): obj is {
    toString(): string;
};
/**
 * Reads a binary stream of ScSpecEntries into an array for processing by ContractSpec
 * @private
 */
export declare function processSpecEntryStream(buffer: Buffer): xdr.ScSpecEntry[];
export declare function getAccount<T>(options: AssembledTransactionOptions<T>, server: Server): Promise<Account>;
