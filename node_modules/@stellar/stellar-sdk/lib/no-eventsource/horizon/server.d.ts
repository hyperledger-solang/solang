import { Asset, FeeBumpTransaction, Transaction } from "@stellar/stellar-base";
import URI from "urijs";
import { AccountCallBuilder } from "./account_call_builder";
import { AccountResponse } from "./account_response";
import { AssetsCallBuilder } from "./assets_call_builder";
import { ClaimableBalanceCallBuilder } from "./claimable_balances_call_builder";
import { EffectCallBuilder } from "./effect_call_builder";
import { FriendbotBuilder } from "./friendbot_builder";
import { HorizonApi } from "./horizon_api";
import { LedgerCallBuilder } from "./ledger_call_builder";
import { LiquidityPoolCallBuilder } from "./liquidity_pool_call_builder";
import { OfferCallBuilder } from "./offer_call_builder";
import { OperationCallBuilder } from "./operation_call_builder";
import { OrderbookCallBuilder } from "./orderbook_call_builder";
import { PathCallBuilder } from "./path_call_builder";
import { PaymentCallBuilder } from "./payment_call_builder";
import { TradeAggregationCallBuilder } from "./trade_aggregation_call_builder";
import { TradesCallBuilder } from "./trades_call_builder";
import { TransactionCallBuilder } from "./transaction_call_builder";
/**
 * Default transaction submission timeout for Horizon requests, in milliseconds
 * @constant {number}
 * @default 60000
 * @memberof module:Horizon.Server
 */
export declare const SUBMIT_TRANSACTION_TIMEOUT: number;
/**
 * Server handles the network connection to a [Horizon](https://developers.stellar.org/docs/data/horizon)
 * instance and exposes an interface for requests to that instance.
 * @class
 * @alias module:Horizon.Server
 * @memberof module:Horizon
 *
 * @param {string} serverURL Horizon Server URL (ex. `https://horizon-testnet.stellar.org`).
 * @param {module:Horizon.Server.Options} [opts] Options object
 */
export declare class HorizonServer {
    /**
     * Horizon Server URL (ex. `https://horizon-testnet.stellar.org`)
     *
     * @todo Solve `URI(this.serverURL as any)`.
     */
    readonly serverURL: URI;
    constructor(serverURL: string, opts?: HorizonServer.Options);
    /**
     * Get timebounds for N seconds from now, when you're creating a transaction
     * with {@link TransactionBuilder}.
     *
     * By default, {@link TransactionBuilder} uses the current local time, but
     * your machine's local time could be different from Horizon's. This gives you
     * more assurance that your timebounds will reflect what you want.
     *
     * Note that this will generate your timebounds when you **init the transaction**,
     * not when you build or submit the transaction! So give yourself enough time to get
     * the transaction built and signed before submitting.
     *
     * @example
     * const transaction = new StellarSdk.TransactionBuilder(accountId, {
     *   fee: await StellarSdk.Server.fetchBaseFee(),
     *   timebounds: await StellarSdk.Server.fetchTimebounds(100)
     * })
     *   .addOperation(operation)
     *   // normally we would need to call setTimeout here, but setting timebounds
     *   // earlier does the trick!
     *   .build();
     *
     * @param {number} seconds Number of seconds past the current time to wait.
     * @param {boolean} [_isRetry] True if this is a retry. Only set this internally!
     * This is to avoid a scenario where Horizon is horking up the wrong date.
     * @returns {Promise<Timebounds>} Promise that resolves a `timebounds` object
     * (with the shape `{ minTime: 0, maxTime: N }`) that you can set the `timebounds` option to.
     */
    fetchTimebounds(seconds: number, _isRetry?: boolean): Promise<HorizonServer.Timebounds>;
    /**
     * Fetch the base fee. Since this hits the server, if the server call fails,
     * you might get an error. You should be prepared to use a default value if
     * that happens!
     * @returns {Promise<number>} Promise that resolves to the base fee.
     */
    fetchBaseFee(): Promise<number>;
    /**
     * Fetch the fee stats endpoint.
     * @see {@link https://developers.stellar.org/docs/data/horizon/api-reference/aggregations/fee-stats|Fee Stats}
     * @returns {Promise<HorizonApi.FeeStatsResponse>} Promise that resolves to the fee stats returned by Horizon.
     */
    feeStats(): Promise<HorizonApi.FeeStatsResponse>;
    /**
     * Fetch the Horizon server's root endpoint.
     * @returns {Promise<HorizonApi.RootResponse>} Promise that resolves to the root endpoint returned by Horizon.
     */
    root(): Promise<HorizonApi.RootResponse>;
    /**
     * Submits a transaction to the network.
     *
     * By default this function calls {@link Horizon.Server#checkMemoRequired}, you can
     * skip this check by setting the option `skipMemoRequiredCheck` to `true`.
     *
     * If you submit any number of `manageOffer` operations, this will add an
     * attribute to the response that will help you analyze what happened with
     * your offers.
     *
     * For example, you'll want to examine `offerResults` to add affordances like
     * these to your app:
     * - If `wasImmediatelyFilled` is true, then no offer was created. So if you
     *   normally watch the `Server.offers` endpoint for offer updates, you
     *   instead need to check `Server.trades` to find the result of this filled
     *   offer.
     * - If `wasImmediatelyDeleted` is true, then the offer you submitted was
     *   deleted without reaching the orderbook or being matched (possibly because
     *   your amounts were rounded down to zero). So treat the just-submitted
     *   offer request as if it never happened.
     * - If `wasPartiallyFilled` is true, you can tell the user that
     *   `amountBought` or `amountSold` have already been transferred.
     *
     * @example
     * const res = {
     *   ...response,
     *   offerResults: [
     *     {
     *       // Exact ordered list of offers that executed, with the exception
     *       // that the last one may not have executed entirely.
     *       offersClaimed: [
     *         sellerId: String,
     *         offerId: String,
     *         assetSold: {
     *           type: 'native|credit_alphanum4|credit_alphanum12',
     *
     *           // these are only present if the asset is not native
     *           assetCode: String,
     *           issuer: String,
     *         },
     *
     *         // same shape as assetSold
     *         assetBought: {}
     *       ],
     *
     *       // What effect your manageOffer op had
     *       effect: "manageOfferCreated|manageOfferUpdated|manageOfferDeleted",
     *
     *       // Whether your offer immediately got matched and filled
     *       wasImmediatelyFilled: Boolean,
     *
     *       // Whether your offer immediately got deleted, if for example the order was too small
     *       wasImmediatelyDeleted: Boolean,
     *
     *       // Whether the offer was partially, but not completely, filled
     *       wasPartiallyFilled: Boolean,
     *
     *       // The full requested amount of the offer is open for matching
     *       isFullyOpen: Boolean,
     *
     *       // The total amount of tokens bought / sold during transaction execution
     *       amountBought: Number,
     *       amountSold: Number,
     *
     *       // if the offer was created, updated, or partially filled, this is
     *       // the outstanding offer
     *       currentOffer: {
     *         offerId: String,
     *         amount: String,
     *         price: {
     *           n: String,
     *           d: String,
     *         },
     *
     *         selling: {
     *           type: 'native|credit_alphanum4|credit_alphanum12',
     *
     *           // these are only present if the asset is not native
     *           assetCode: String,
     *           issuer: String,
     *         },
     *
     *         // same as `selling`
     *         buying: {},
     *       },
     *
     *       // the index of this particular operation in the op stack
     *       operationIndex: Number
     *     }
     *   ]
     * }
     *
     * @see {@link https://developers.stellar.org/docs/data/horizon/api-reference/resources/submit-a-transaction|Submit a Transaction}
     * @param {Transaction|FeeBumpTransaction} transaction - The transaction to submit.
     * @param {object} [opts] Options object
     * @param {boolean} [opts.skipMemoRequiredCheck] - Allow skipping memo
     * required check, default: `false`. See
     * [SEP0029](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0029.md).
     * @returns {Promise} Promise that resolves or rejects with response from
     * horizon.
     */
    submitTransaction(transaction: Transaction | FeeBumpTransaction, opts?: HorizonServer.SubmitTransactionOptions): Promise<HorizonApi.SubmitTransactionResponse>;
    /**
     * Submits an asynchronous transaction to the network. Unlike the synchronous version, which blocks
     * and waits for the transaction to be ingested in Horizon, this endpoint relays the response from
     * core directly back to the user.
     *
     * By default, this function calls {@link HorizonServer#checkMemoRequired}, you can
     * skip this check by setting the option `skipMemoRequiredCheck` to `true`.
     *
     * @see [Submit-Async-Transaction](https://developers.stellar.org/docs/data/horizon/api-reference/resources/submit-async-transaction)
     * @param {Transaction|FeeBumpTransaction} transaction - The transaction to submit.
     * @param {object} [opts] Options object
     * @param {boolean} [opts.skipMemoRequiredCheck] - Allow skipping memo
     * required check, default: `false`. See
     * [SEP0029](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0029.md).
     * @returns {Promise} Promise that resolves or rejects with response from
     * horizon.
     */
    submitAsyncTransaction(transaction: Transaction | FeeBumpTransaction, opts?: HorizonServer.SubmitTransactionOptions): Promise<HorizonApi.SubmitAsyncTransactionResponse>;
    /**
     * @returns {AccountCallBuilder} New {@link AccountCallBuilder} object configured by a current Horizon server configuration.
     */
    accounts(): AccountCallBuilder;
    /**
     * @returns {ClaimableBalanceCallBuilder} New {@link ClaimableBalanceCallBuilder} object configured by a current Horizon server configuration.
     */
    claimableBalances(): ClaimableBalanceCallBuilder;
    /**
     * @returns {LedgerCallBuilder} New {@link LedgerCallBuilder} object configured by a current Horizon server configuration.
     */
    ledgers(): LedgerCallBuilder;
    /**
     * @returns {TransactionCallBuilder} New {@link TransactionCallBuilder} object configured by a current Horizon server configuration.
     */
    transactions(): TransactionCallBuilder;
    /**
     * People on the Stellar network can make offers to buy or sell assets. This endpoint represents all the offers on the DEX.
     *
     * You can query all offers for account using the function `.accountId`.
     *
     * @example
     * server.offers()
     *   .forAccount(accountId).call()
     *   .then(function(offers) {
     *     console.log(offers);
     *   });
     *
     * @returns {OfferCallBuilder} New {@link OfferCallBuilder} object
     */
    offers(): OfferCallBuilder;
    /**
     * @param {Asset} selling Asset being sold
     * @param {Asset} buying Asset being bought
     * @returns {OrderbookCallBuilder} New {@link OrderbookCallBuilder} object configured by a current Horizon server configuration.
     */
    orderbook(selling: Asset, buying: Asset): OrderbookCallBuilder;
    /**
     * Returns
     * @returns {TradesCallBuilder} New {@link TradesCallBuilder} object configured by a current Horizon server configuration.
     */
    trades(): TradesCallBuilder;
    /**
     * @returns {OperationCallBuilder} New {@link OperationCallBuilder} object configured by a current Horizon server configuration.
     */
    operations(): OperationCallBuilder;
    /**
     * @returns {LiquidityPoolCallBuilder} New {@link LiquidityPoolCallBuilder}
     *     object configured to the current Horizon server settings.
     */
    liquidityPools(): LiquidityPoolCallBuilder;
    /**
     * The Stellar Network allows payments to be made between assets through path
     * payments. A strict receive path payment specifies a series of assets to
     * route a payment through, from source asset (the asset debited from the
     * payer) to destination asset (the asset credited to the payee).
     *
     * A strict receive path search is specified using:
     *
     * * The destination address.
     * * The source address or source assets.
     * * The asset and amount that the destination account should receive.
     *
     * As part of the search, horizon will load a list of assets available to the
     * source address and will find any payment paths from those source assets to
     * the desired destination asset. The search's amount parameter will be used
     * to determine if there a given path can satisfy a payment of the desired
     * amount.
     *
     * If a list of assets is passed as the source, horizon will find any payment
     * paths from those source assets to the desired destination asset.
     *
     * @param {string|Asset[]} source The sender's account ID or a list of assets. Any returned path will use a source that the sender can hold.
     * @param {Asset} destinationAsset The destination asset.
     * @param {string} destinationAmount The amount, denominated in the destination asset, that any returned path should be able to satisfy.
     * @returns {StrictReceivePathCallBuilder} New {@link StrictReceivePathCallBuilder} object configured with the current Horizon server configuration.
     */
    strictReceivePaths(source: string | Asset[], destinationAsset: Asset, destinationAmount: string): PathCallBuilder;
    /**
     * The Stellar Network allows payments to be made between assets through path payments. A strict send path payment specifies a
     * series of assets to route a payment through, from source asset (the asset debited from the payer) to destination
     * asset (the asset credited to the payee).
     *
     * A strict send path search is specified using:
     *
     * The asset and amount that is being sent.
     * The destination account or the destination assets.
     *
     * @param {Asset} sourceAsset The asset to be sent.
     * @param {string} sourceAmount The amount, denominated in the source asset, that any returned path should be able to satisfy.
     * @param {string|Asset[]} destination The destination account or the destination assets.
     * @returns {StrictSendPathCallBuilder} New {@link StrictSendPathCallBuilder} object configured with the current Horizon server configuration.
     */
    strictSendPaths(sourceAsset: Asset, sourceAmount: string, destination: string | Asset[]): PathCallBuilder;
    /**
     * @returns {PaymentCallBuilder} New {@link PaymentCallBuilder} instance configured with the current
     * Horizon server configuration.
     */
    payments(): PaymentCallBuilder;
    /**
     * @returns {EffectCallBuilder} New {@link EffectCallBuilder} instance configured with the current
     * Horizon server configuration
     */
    effects(): EffectCallBuilder;
    /**
     * @param {string} address The Stellar ID that you want Friendbot to send lumens to
     * @returns {FriendbotBuilder} New {@link FriendbotBuilder} instance configured with the current
     * Horizon server configuration
     * @private
     */
    friendbot(address: string): FriendbotBuilder;
    /**
     * Get a new {@link AssetsCallBuilder} instance configured with the current
     * Horizon server configuration.
     * @returns {AssetsCallBuilder} New AssetsCallBuilder instance
     */
    assets(): AssetsCallBuilder;
    /**
     * Fetches an account's most current state in the ledger, then creates and
     * returns an {@link AccountResponse} object.
     *
     * @param {string} accountId - The account to load.
     *
     * @returns {Promise} Returns a promise to the {@link AccountResponse} object
     * with populated sequence number.
     */
    loadAccount(accountId: string): Promise<AccountResponse>;
    /**
     *
     * @param {Asset} base base asset
     * @param {Asset} counter counter asset
     * @param {number} start_time lower time boundary represented as millis since epoch
     * @param {number} end_time upper time boundary represented as millis since epoch
     * @param {number} resolution segment duration as millis since epoch. *Supported values are 5 minutes (300000), 15 minutes (900000), 1 hour (3600000), 1 day (86400000) and 1 week (604800000).
     * @param {number} offset segments can be offset using this parameter. Expressed in milliseconds. *Can only be used if the resolution is greater than 1 hour. Value must be in whole hours, less than the provided resolution, and less than 24 hours.
     * Returns new {@link TradeAggregationCallBuilder} object configured with the current Horizon server configuration.
     * @returns {TradeAggregationCallBuilder} New TradeAggregationCallBuilder instance
     */
    tradeAggregation(base: Asset, counter: Asset, start_time: number, end_time: number, resolution: number, offset: number): TradeAggregationCallBuilder;
    /**
     * Check if any of the destination accounts requires a memo.
     *
     * This function implements a memo required check as defined in
     * [SEP-29](https://stellar.org/protocol/sep-29). It will load each account
     * which is the destination and check if it has the data field
     * `config.memo_required` set to `"MQ=="`.
     *
     * Each account is checked sequentially instead of loading multiple accounts
     * at the same time from Horizon.
     *
     * @see {@link https://stellar.org/protocol/sep-29|SEP-29: Account Memo Requirements}
     * @param {Transaction} transaction - The transaction to check.
     * @returns {Promise<void, Error>} - If any of the destination account
     * requires a memo, the promise will throw {@link AccountRequiresMemoError}.
     * @throws  {AccountRequiresMemoError}
     */
    checkMemoRequired(transaction: Transaction | FeeBumpTransaction): Promise<void>;
}
/**
 * Options for configuring connections to Horizon servers.
 * @typedef {object} Options
 * @memberof module:Horizon.Server
 * @property {boolean} [allowHttp] Allow connecting to http servers, default: `false`. This must be set to false in production deployments! You can also use {@link Config} class to set this globally.
 * @property {string} [appName] Allow set custom header `X-App-Name`, default: `undefined`.
 * @property {string} [appVersion] Allow set custom header `X-App-Version`, default: `undefined`.
 * @property {string} [authToken] Allow set custom header `X-Auth-Token`, default: `undefined`.
 */
export declare namespace HorizonServer {
    interface Options {
        allowHttp?: boolean;
        appName?: string;
        appVersion?: string;
        authToken?: string;
        headers?: Record<string, string>;
    }
    interface Timebounds {
        minTime: number;
        maxTime: number;
    }
    interface SubmitTransactionOptions {
        skipMemoRequiredCheck?: boolean;
    }
}
