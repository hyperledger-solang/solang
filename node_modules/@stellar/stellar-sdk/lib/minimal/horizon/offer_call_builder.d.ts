import { Asset } from "@stellar/stellar-base";
import { CallBuilder } from "./call_builder";
import { ServerApi } from "./server_api";
/**
 * Creates a new {@link OfferCallBuilder} pointed to server defined by serverUrl.
 *
 * Do not create this object directly, use {@link Horizon.Server#offers}.
 *
 * @see {@link https://developers.stellar.org/docs/data/horizon/api-reference/resources/offers/|Offers}
 *
 * @augments CallBuilder
 * @private
 * @class
 * @param {string} serverUrl Horizon server URL.
 */
export declare class OfferCallBuilder extends CallBuilder<ServerApi.CollectionPage<ServerApi.OfferRecord>> {
    constructor(serverUrl: URI);
    /**
     * The offer details endpoint provides information on a single offer. The offer ID provided in the id
     * argument specifies which offer to load.
     * @see {@link https://developers.stellar.org/docs/data/horizon/api-reference/resources/offers/single/|Offer Details}
     * @param {string} offerId Offer ID
     * @returns {CallBuilder<ServerApi.OfferRecord>} CallBuilder<ServerApi.OfferRecord> OperationCallBuilder instance
     */
    offer(offerId: string): CallBuilder<ServerApi.OfferRecord>;
    /**
     * Returns all offers where the given account is involved.
     *
     * @see {@link https://developers.stellar.org/docs/data/horizon/api-reference/resources/accounts/offers/|Offers}
     * @param {string} id For example: `GDGQVOKHW4VEJRU2TETD6DBRKEO5ERCNF353LW5WBFW3JJWQ2BRQ6KDD`
     * @returns {OfferCallBuilder} current OfferCallBuilder instance
     */
    forAccount(id: string): this;
    /**
     * Returns all offers buying an asset.
     * @see {@link https://developers.stellar.org/docs/data/horizon/api-reference/resources/offers/list/|Offers}
     * @see Asset
     * @param {Asset} asset For example: `new Asset('USD','GDGQVOKHW4VEJRU2TETD6DBRKEO5ERCNF353LW5WBFW3JJWQ2BRQ6KDD')`
     * @returns {OfferCallBuilder} current OfferCallBuilder instance
     */
    buying(asset: Asset): this;
    /**
     * Returns all offers selling an asset.
     * @see {@link https://developers.stellar.org/docs/data/horizon/api-reference/resources/offers/list/|Offers}
     * @see Asset
     * @param {Asset} asset For example: `new Asset('EUR','GDGQVOKHW4VEJRU2TETD6DBRKEO5ERCNF353LW5WBFW3JJWQ2BRQ6KDD')`
     * @returns {OfferCallBuilder} current OfferCallBuilder instance
     */
    selling(asset: Asset): this;
    /**
     * This endpoint filters offers where the given account is sponsoring the offer entry.
     * @see {@link https://developers.stellar.org/docs/data/horizon/api-reference/resources/get-all-offers|Offers}
     * @param {string} id For example: `GDGQVOKHW4VEJRU2TETD6DBRKEO5ERCNF353LW5WBFW3JJWQ2BRQ6KDD`
     * @returns {OfferCallBuilder} current OfferCallBuilder instance
     */
    sponsor(id: string): this;
    /**
     * This endpoint filters offers where the given account is the seller.
     *
     * @see {@link https://developers.stellar.org/docs/data/horizon/api-reference/resources/get-all-offers|Offers}
     * @param {string} seller For example: `GDGQVOKHW4VEJRU2TETD6DBRKEO5ERCNF353LW5WBFW3JJWQ2BRQ6KDD`
     * @returns {OfferCallBuilder} current OfferCallBuilder instance
     */
    seller(seller: string): this;
}
