import { FeeBumpTransaction, Transaction, TransactionBuilder } from '@stellar/stellar-base';
import { Api } from './api';
/**
 * Combines the given raw transaction alongside the simulation results.
 * If the given transaction already has authorization entries in a host
 * function invocation (see {@link Operation.invokeHostFunction}), **the
 * simulation entries are ignored**.
 *
 * If the given transaction already has authorization entries in a host function
 * invocation (see {@link Operation.invokeHostFunction}), **the simulation
 * entries are ignored**.
 *
 * @param {Transaction|FeeBumpTransaction} raw the initial transaction, w/o simulation applied
 * @param {Api.SimulateTransactionResponse|Api.RawSimulateTransactionResponse} simulation the Soroban RPC simulation result (see {@link module:rpc.Server#simulateTransaction})
 * @returns {TransactionBuilder} a new, cloned transaction with the proper auth and resource (fee, footprint) simulation data applied
 *
 * @memberof module:rpc
 * @see {@link module:rpc.Server#simulateTransaction}
 * @see {@link module:rpc.Server#prepareTransaction}
 */
export declare function assembleTransaction(raw: Transaction | FeeBumpTransaction, simulation: Api.SimulateTransactionResponse | Api.RawSimulateTransactionResponse): TransactionBuilder;
