import { Asset } from "@stellar/stellar-base";
import { CallBuilder } from "./call_builder";
import { ServerApi } from "./server_api";
/**
 * The Stellar Network allows payments to be made across assets through path
 * payments. A strict receive path payment specifies a series of assets to route
 * a payment through, from source asset (the asset debited from the payer) to
 * destination asset (the asset credited to the payee).
 *
 * A path search is specified using:
 *
 * * The source address or source assets.
 * * The asset and amount that the destination account should receive
 *
 * As part of the search, horizon will load a list of assets available to the
 * source address and will find any payment paths from those source assets to
 * the desired destination asset. The search's amount parameter will be used to
 * determine if there a given path can satisfy a payment of the desired amount.
 *
 * If a list of assets is passed as the source, horizon will find any payment
 * paths from those source assets to the desired destination asset.
 *
 * Do not create this object directly, use {@link Horizon.Server#strictReceivePaths}.
 *
 * @see {@link https://developers.stellar.org/docs/data/horizon/api-reference/aggregations/paths|Find Payment Paths}
 *
 * @augments CallBuilder
 * @private
 * @class
 *
 * @param {string} serverUrl Horizon server URL.
 * @param {string|Asset[]} source The sender's account ID or a list of Assets. Any returned path must use a source that the sender can hold.
 * @param {Asset} destinationAsset The destination asset.
 * @param {string} destinationAmount The amount, denominated in the destination asset, that any returned path should be able to satisfy.
 */
export declare class StrictReceivePathCallBuilder extends CallBuilder<ServerApi.CollectionPage<ServerApi.PaymentPathRecord>> {
    constructor(serverUrl: URI, source: string | Asset[], destinationAsset: Asset, destinationAmount: string);
}
