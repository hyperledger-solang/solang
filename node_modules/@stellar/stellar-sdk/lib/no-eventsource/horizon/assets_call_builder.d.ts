import { CallBuilder } from "./call_builder";
import { ServerApi } from "./server_api";
/**
 * Creates a new {@link AssetsCallBuilder} pointed to server defined by serverUrl.
 *
 * Do not create this object directly, use {@link Horizon.Server#assets}.
 *
 * @class
 * @augments CallBuilder
 * @private
 * @param {string} serverUrl Horizon server URL.
 */
export declare class AssetsCallBuilder extends CallBuilder<ServerApi.CollectionPage<ServerApi.AssetRecord>> {
    constructor(serverUrl: URI);
    /**
     * This endpoint filters all assets by the asset code.
     * @param {string} value For example: `USD`
     * @returns {AssetsCallBuilder} current AssetCallBuilder instance
     */
    forCode(value: string): AssetsCallBuilder;
    /**
     * This endpoint filters all assets by the asset issuer.
     * @param {string} value For example: `GDGQVOKHW4VEJRU2TETD6DBRKEO5ERCNF353LW5WBFW3JJWQ2BRQ6KDD`
     * @returns {AssetsCallBuilder} current AssetCallBuilder instance
     */
    forIssuer(value: string): AssetsCallBuilder;
}
