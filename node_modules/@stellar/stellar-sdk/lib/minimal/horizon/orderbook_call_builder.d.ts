import { Asset } from "@stellar/stellar-base";
import { CallBuilder } from "./call_builder";
import { ServerApi } from "./server_api";
/**
 * Creates a new {@link OrderbookCallBuilder} pointed to server defined by serverUrl.
 *
 * Do not create this object directly, use {@link Horizon.Server#orderbook}.
 *
 * @see {@link https://developers.stellar.org/docs/data/horizon/api-reference/aggregations/order-books|Orderbook Details}
 *
 * @augments CallBuilder
 * @private
 * @class
 * @param {string} serverUrl serverUrl Horizon server URL.
 * @param {Asset} selling Asset being sold
 * @param {Asset} buying Asset being bought
 */
export declare class OrderbookCallBuilder extends CallBuilder<ServerApi.OrderbookRecord> {
    constructor(serverUrl: URI, selling: Asset, buying: Asset);
}
