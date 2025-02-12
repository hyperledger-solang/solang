import { Asset } from "@stellar/stellar-base";
import { CallBuilder } from "./call_builder";
import { ServerApi } from "./server_api";
/**
 * The Stellar Network allows payments to be made across assets through path payments. A path payment specifies a
 * series of assets to route a payment through, from source asset (the asset debited from the payer) to destination
 * asset (the asset credited to the payee).
 *
 * A path search is specified using:
 *
 * * The destination address
 * * The source address
 * * The asset and amount that the destination account should receive
 *
 * As part of the search, horizon will load a list of assets available to the source address and will find any
 * payment paths from those source assets to the desired destination asset. The search's amount parameter will be
 * used to determine if there a given path can satisfy a payment of the desired amount.
 *
 * Do not create this object directly, use {@link Horizon.Server#paths}.
 *
 * @see {@link https://developers.stellar.org/docs/data/horizon/api-reference/aggregations/paths|Find Payment Paths}
 *
 * @augments CallBuilder
 * @private
 * @class
 *
 * @param {string} serverUrl Horizon server URL.
 * @param {string} source The sender's account ID. Any returned path must use a source that the sender can hold.
 * @param {string} destination The destination account ID that any returned path should use.
 * @param {Asset} destinationAsset The destination asset.
 * @param {string} destinationAmount The amount, denominated in the destination asset, that any returned path should be able to satisfy.
 */
export declare class PathCallBuilder extends CallBuilder<ServerApi.CollectionPage<ServerApi.PaymentPathRecord>> {
    constructor(serverUrl: URI, source: string, destination: string, destinationAsset: Asset, destinationAmount: string);
}
