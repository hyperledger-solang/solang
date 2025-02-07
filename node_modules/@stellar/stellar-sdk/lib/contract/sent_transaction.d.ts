import { Server } from "../rpc";
import { Api } from "../rpc/api";
import type { AssembledTransaction } from "./assembled_transaction";
/**
 * A transaction that has been sent to the Soroban network. This happens in two steps:
 *
 * 1. `sendTransaction`: initial submission of the transaction to the network.
 *    If this step runs into problems, the attempt to sign and send will be
 *    aborted. You can see the result of this call in the
 *    `sendTransactionResponse` getter.
 * 2. `getTransaction`: once the transaction has been submitted to the network
 *    successfully, you need to wait for it to finalize to get the result of the
 *    transaction. This will be retried with exponential backoff for
 *    {@link MethodOptions.timeoutInSeconds} seconds. See all attempts in
 *    `getTransactionResponseAll` and the most recent attempt in
 *    `getTransactionResponse`.
 *
 * @memberof module:contract
 * @class
 *
 * @param {Function} signTransaction More info in {@link MethodOptions}
 * @param {module:contract.AssembledTransaction<T>} assembled {@link AssembledTransaction} from which this SentTransaction was initialized
 */
export declare class SentTransaction<T> {
    assembled: AssembledTransaction<T>;
    server: Server;
    /**
     * The result of calling `sendTransaction` to broadcast the transaction to the
     * network.
     */
    sendTransactionResponse?: Api.SendTransactionResponse;
    /**
     * If `sendTransaction` completes successfully (which means it has `status: 'PENDING'`),
     * then `getTransaction` will be called in a loop for
     * {@link MethodOptions.timeoutInSeconds} seconds. This array contains all
     * the results of those calls.
     */
    getTransactionResponseAll?: Api.GetTransactionResponse[];
    /**
     * The most recent result of calling `getTransaction`, from the
     * `getTransactionResponseAll` array.
     */
    getTransactionResponse?: Api.GetTransactionResponse;
    static Errors: {
        SendFailed: {
            new (message?: string): {
                name: string;
                message: string;
                stack?: string;
            };
            captureStackTrace(targetObject: object, constructorOpt?: Function): void;
            prepareStackTrace?: ((err: Error, stackTraces: NodeJS.CallSite[]) => any) | undefined;
            stackTraceLimit: number;
        };
        SendResultOnly: {
            new (message?: string): {
                name: string;
                message: string;
                stack?: string;
            };
            captureStackTrace(targetObject: object, constructorOpt?: Function): void;
            prepareStackTrace?: ((err: Error, stackTraces: NodeJS.CallSite[]) => any) | undefined;
            stackTraceLimit: number;
        };
        TransactionStillPending: {
            new (message?: string): {
                name: string;
                message: string;
                stack?: string;
            };
            captureStackTrace(targetObject: object, constructorOpt?: Function): void;
            prepareStackTrace?: ((err: Error, stackTraces: NodeJS.CallSite[]) => any) | undefined;
            stackTraceLimit: number;
        };
    };
    constructor(assembled: AssembledTransaction<T>);
    /**
     * Initialize a `SentTransaction` from `options` and a `signed`
     * AssembledTransaction. This will also send the transaction to the network.
     */
    static init: <U>(assembled: AssembledTransaction<U>) => Promise<SentTransaction<U>>;
    private send;
    get result(): T;
}
