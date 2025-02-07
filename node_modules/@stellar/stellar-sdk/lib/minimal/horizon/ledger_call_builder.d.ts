import { CallBuilder } from "./call_builder";
import { ServerApi } from "./server_api";
/**
 * Creates a new {@link LedgerCallBuilder} pointed to server defined by serverUrl.
 *
 * Do not create this object directly, use {@link Horizon.Server#ledgers}.
 *
 * @see {@link https://developers.stellar.org/docs/data/horizon/api-reference/resources/list-all-ledgers|All Ledgers}
 *
 * @augments CallBuilder
 * @private
 * @class
 * @param {string} serverUrl Horizon server URL.
 */
export declare class LedgerCallBuilder extends CallBuilder<ServerApi.CollectionPage<ServerApi.LedgerRecord>> {
    constructor(serverUrl: URI);
    /**
     * Provides information on a single ledger.
     * @param {number|string} sequence Ledger sequence
     * @returns {LedgerCallBuilder} current LedgerCallBuilder instance
     */
    ledger(sequence: number | string): this;
}
