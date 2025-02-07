import { Api } from './api';
/**
 * Parse the response from invoking the `submitTransaction` method of a Soroban RPC server.
 * @memberof module:rpc
 * @private
 *
 * @param {Api.RawSendTransactionResponse} raw the raw `submitTransaction` response from the Soroban RPC server to parse
 * @returns {Api.SendTransactionResponse} transaction response parsed from the Soroban RPC server's response
 */
export declare function parseRawSendTransaction(raw: Api.RawSendTransactionResponse): Api.SendTransactionResponse;
export declare function parseTransactionInfo(raw: Api.RawTransactionInfo | Api.RawGetTransactionResponse): Omit<Api.TransactionInfo, 'status' | 'txHash'>;
export declare function parseRawTransactions(r: Api.RawTransactionInfo): Api.TransactionInfo;
/**
 * Parse and return the retrieved events, if any, from a raw response from a Soroban RPC server.
 * @memberof module:rpc
 *
 * @param {Api.RawGetEventsResponse} raw the raw `getEvents` response from the Soroban RPC server to parse
 * @returns {Api.GetEventsResponse} events parsed from the Soroban RPC server's response
 */
export declare function parseRawEvents(raw: Api.RawGetEventsResponse): Api.GetEventsResponse;
/**
 * Parse and return the retrieved ledger entries, if any, from a raw response from a Soroban RPC server.
 * @memberof module:rpc
 * @private
 *
 * @param {Api.RawGetLedgerEntriesResponse} raw he raw `getLedgerEntries` response from the Soroban RPC server to parse
 * @returns {Api.GetLedgerEntriesResponse} ledger entries parsed from the Soroban RPC server's response
 */
export declare function parseRawLedgerEntries(raw: Api.RawGetLedgerEntriesResponse): Api.GetLedgerEntriesResponse;
/**
 * Converts a raw response schema into one with parsed XDR fields and a simplified interface.
 * @warning This API is only exported for testing purposes and should not be relied on or considered "stable".
 * @memberof module:rpc
 *
 * @param {Api.SimulateTransactionResponse | Api.RawSimulateTransactionResponse} sim the raw response schema (parsed ones are allowed, best-effort
 *    detected, and returned untouched)
 * @returns {Api.SimulateTransactionResponse} the original parameter (if already parsed), parsed otherwise
 */
export declare function parseRawSimulation(sim: Api.SimulateTransactionResponse | Api.RawSimulateTransactionResponse): Api.SimulateTransactionResponse;
