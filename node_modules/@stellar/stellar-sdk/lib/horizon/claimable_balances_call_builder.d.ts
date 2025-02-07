import { Asset } from "@stellar/stellar-base";
import { CallBuilder } from "./call_builder";
import { ServerApi } from "./server_api";
/**
 * Creates a new {@link ClaimableBalanceCallBuilder} pointed to server defined by serverUrl.
 *
 * Do not create this object directly, use {@link Horizon.Server#claimableBalances}.
 *
 * @see {@link https://developers.stellar.org/docs/data/horizon/api-reference/resources/claimablebalances|Claimable Balances}
 *
 * @augments CallBuilder
 * @private
 * @class
 * @param {string} serverUrl Horizon server URL.
 */
export declare class ClaimableBalanceCallBuilder extends CallBuilder<ServerApi.CollectionPage<ServerApi.ClaimableBalanceRecord>> {
    constructor(serverUrl: URI);
    /**
     * The claimable balance details endpoint provides information on a single claimable balance.
     *
     * @see {@link https://developers.stellar.org/docs/data/horizon/api-reference/resources/retrieve-a-claimable-balance|Claimable Balance Details}
     * @param {string} claimableBalanceId Claimable balance ID
     * @returns {CallBuilder<ServerApi.ClaimableBalanceRecord>} CallBuilder<ServerApi.ClaimableBalanceRecord> OperationCallBuilder instance
     */
    claimableBalance(claimableBalanceId: string): CallBuilder<ServerApi.ClaimableBalanceRecord>;
    /**
     * Returns all claimable balances which are sponsored by the given account ID.
     *
     * @see {@link https://developers.stellar.org/docs/data/horizon/api-reference/resources/list-all-claimable-balances|Claimable Balances}
     * @param {string} sponsor For example: `GDGQVOKHW4VEJRU2TETD6DBRKEO5ERCNF353LW5WBFW3JJWQ2BRQ6KDD`
     * @returns {ClaimableBalanceCallBuilder} current ClaimableBalanceCallBuilder instance
     */
    sponsor(sponsor: string): this;
    /**
     * Returns all claimable balances which can be claimed by the given account ID.
     *
     * @see {@link https://developers.stellar.org/docs/data/horizon/api-reference/resources/list-all-claimable-balances|Claimable Balances}
     * @param {string} claimant For example: `GDGQVOKHW4VEJRU2TETD6DBRKEO5ERCNF353LW5WBFW3JJWQ2BRQ6KDD`
     * @returns {ClaimableBalanceCallBuilder} current ClaimableBalanceCallBuilder instance
     */
    claimant(claimant: string): this;
    /**
     * Returns all claimable balances which provide a balance for the given asset.
     *
     * @see {@link https://developers.stellar.org/docs/data/horizon/api-reference/resources/list-all-claimable-balances|Claimable Balances}
     * @param {Asset} asset The Asset held by the claimable balance
     * @returns {ClaimableBalanceCallBuilder} current ClaimableBalanceCallBuilder instance
     */
    asset(asset: Asset): this;
}
