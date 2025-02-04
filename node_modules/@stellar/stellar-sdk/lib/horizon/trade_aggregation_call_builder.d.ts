import { Asset } from "@stellar/stellar-base";
import { CallBuilder } from "./call_builder";
import { HorizonApi } from "./horizon_api";
import { ServerApi } from "./server_api";
/**
 * Trade Aggregations facilitate efficient gathering of historical trade data.
 *
 * Do not create this object directly, use {@link Horizon.Server#tradeAggregation}.
 *
 * @augments CallBuilder
 * @private
 * @class
 *
 * @param {string} serverUrl serverUrl Horizon server URL.
 * @param {Asset} base base asset
 * @param {Asset} counter counter asset
 * @param {number} start_time lower time boundary represented as millis since epoch
 * @param {number} end_time upper time boundary represented as millis since epoch
 * @param {number} resolution segment duration as millis since epoch. *Supported values are 1 minute (60000), 5 minutes (300000), 15 minutes (900000), 1 hour (3600000), 1 day (86400000) and 1 week (604800000).
 * @param {number} offset segments can be offset using this parameter. Expressed in milliseconds. *Can only be used if the resolution is greater than 1 hour. Value must be in whole hours, less than the provided resolution, and less than 24 hours.
 */
export declare class TradeAggregationCallBuilder extends CallBuilder<ServerApi.CollectionPage<TradeAggregationRecord>> {
    constructor(serverUrl: URI, base: Asset, counter: Asset, start_time: number, end_time: number, resolution: number, offset: number);
    /**
     * @private
     * @param {number} resolution Trade data resolution in milliseconds
     * @returns {boolean} true if the resolution is allowed
     */
    private isValidResolution;
    /**
     * @private
     * @param {number} offset Time offset in milliseconds
     * @param {number} resolution Trade data resolution in milliseconds
     * @returns {boolean} true if the offset is valid
     */
    private isValidOffset;
}
interface TradeAggregationRecord extends HorizonApi.BaseResponse {
    timestamp: number | string;
    trade_count: number | string;
    base_volume: string;
    counter_volume: string;
    avg: string;
    high: string;
    low: string;
    open: string;
    close: string;
}
export {};
