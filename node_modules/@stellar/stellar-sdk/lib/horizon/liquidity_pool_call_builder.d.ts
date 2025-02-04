import { Asset } from "@stellar/stellar-base";
import { CallBuilder } from "./call_builder";
import { ServerApi } from "./server_api";
/**
 * Creates a new {@link LiquidityPoolCallBuilder} pointed to server defined by serverUrl.
 *
 * Do not create this object directly, use {@link Horizon.Server#liquidityPools}.
 *
 * @augments CallBuilder
 * @private
 * @class
 * @param {string} serverUrl Horizon server URL.
 */
export declare class LiquidityPoolCallBuilder extends CallBuilder<ServerApi.CollectionPage<ServerApi.LiquidityPoolRecord>> {
    constructor(serverUrl: URI);
    /**
     * Filters out pools whose reserves don't exactly match these assets.
     *
     * @see Asset
     * @returns {LiquidityPoolCallBuilder} current LiquidityPoolCallBuilder instance
     */
    forAssets(...assets: Asset[]): this;
    /**
     * Retrieves all pools an account is participating in.
     *
     * @param {string} id   the participant account to filter by
     * @returns {LiquidityPoolCallBuilder} current LiquidityPoolCallBuilder instance
     */
    forAccount(id: string): this;
    /**
     * Retrieves a specific liquidity pool by ID.
     *
     * @param {string} id   the hash/ID of the liquidity pool
     * @returns {CallBuilder} a new CallBuilder instance for the /liquidity_pools/:id endpoint
     */
    liquidityPoolId(id: string): CallBuilder<ServerApi.LiquidityPoolRecord>;
}
