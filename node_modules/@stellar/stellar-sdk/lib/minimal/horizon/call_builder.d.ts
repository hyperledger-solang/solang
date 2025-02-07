import URI from "urijs";
import { HorizonApi } from "./horizon_api";
import { ServerApi } from "./server_api";
export interface EventSourceOptions<T> {
    onmessage?: (value: T) => void;
    onerror?: (event: MessageEvent) => void;
    reconnectTimeout?: number;
}
/**
 * Creates a new {@link CallBuilder} pointed to server defined by serverUrl.
 *
 * This is an **abstract** class. Do not create this object directly, use {@link Server} class.
 * @param {string} serverUrl URL of Horizon server
 * @class CallBuilder
 */
export declare class CallBuilder<T extends HorizonApi.FeeStatsResponse | HorizonApi.BaseResponse | HorizonApi.RootResponse | ServerApi.CollectionPage<HorizonApi.BaseResponse>> {
    protected url: URI;
    filter: string[][];
    protected originalSegments: string[];
    protected neighborRoot: string;
    constructor(serverUrl: URI, neighborRoot?: string);
    /**
     * Triggers a HTTP request using this builder's current configuration.
     * @returns {Promise} a Promise that resolves to the server's response.
     */
    call(): Promise<T>;
    /**
     * Creates an EventSource that listens for incoming messages from the server. To stop listening for new
     * events call the function returned by this method.
     * @see [Horizon Response Format](https://developers.stellar.org/api/introduction/response-format/)
     * @see [MDN EventSource](https://developer.mozilla.org/en-US/docs/Web/API/EventSource)
     * @param {object} [options] EventSource options.
     * @param {Function} [options.onmessage] Callback function to handle incoming messages.
     * @param {Function} [options.onerror] Callback function to handle errors.
     * @param {number} [options.reconnectTimeout] Custom stream connection timeout in ms, default is 15 seconds.
     * @returns {Function} Close function. Run to close the connection and stop listening for new events.
     */
    stream(options?: EventSourceOptions<T extends ServerApi.CollectionPage<infer U> ? U : T>): () => void;
    /**
     * Sets `cursor` parameter for the current call. Returns the CallBuilder object on which this method has been called.
     * @see [Paging](https://developers.stellar.org/api/introduction/pagination/)
     * @param {string} cursor A cursor is a value that points to a specific location in a collection of resources.
     * @returns {object} current CallBuilder instance
     */
    cursor(cursor: string): this;
    /**
     * Sets `limit` parameter for the current call. Returns the CallBuilder object on which this method has been called.
     * @see [Paging](https://developers.stellar.org/api/introduction/pagination/)
     * @param {number} recordsNumber Number of records the server should return.
     * @returns {object} current CallBuilder instance
     */
    limit(recordsNumber: number): this;
    /**
     * Sets `order` parameter for the current call. Returns the CallBuilder object on which this method has been called.
     * @param {"asc"|"desc"} direction Sort direction
     * @returns {object} current CallBuilder instance
     */
    order(direction: "asc" | "desc"): this;
    /**
     * Sets `join` parameter for the current call. The `join` parameter
     * includes the requested resource in the response. Currently, the
     * only valid value for the parameter is `transactions` and is only
     * supported on the operations and payments endpoints. The response
     * will include a `transaction` field for each operation in the
     * response.
     *
     * @param "include" join Records to be included in the response.
     * @returns {object} current CallBuilder instance.
     */
    join(include: "transactions"): this;
    /**
     * A helper method to craft queries to "neighbor" endpoints.
     *
     *  For example, we have an `/effects` suffix endpoint on many different
     *  "root" endpoints, such as `/transactions/:id` and `/accounts/:id`. So,
     *  it's helpful to be able to conveniently create queries to the
     *  `/accounts/:id/effects` endpoint:
     *
     *    this.forEndpoint("accounts", accountId)`.
     *
     * @param  {string} endpoint neighbor endpoint in question, like /operations
     * @param  {string} param    filter parameter, like an operation ID
     *
     * @returns {CallBuilder} this CallBuilder instance
     */
    protected forEndpoint(endpoint: string, param: string): this;
    /**
     * @private
     * @returns {void}
     */
    private checkFilter;
    /**
     * Convert a link object to a function that fetches that link.
     * @private
     * @param {object} link A link object
     * @param {boolean} link.href the URI of the link
     * @param {boolean} [link.templated] Whether the link is templated
     * @returns {Function} A function that requests the link
     */
    private _requestFnForLink;
    /**
     * Given the json response, find and convert each link into a function that
     * calls that link.
     * @private
     * @param {object} json JSON response
     * @returns {object} JSON response with string links replaced with functions
     */
    private _parseRecord;
    private _sendNormalRequest;
    /**
     * @private
     * @param {object} json Response object
     * @returns {object} Extended response
     */
    private _parseResponse;
    /**
     * @private
     * @param {object} json Response object
     * @returns {object} Extended response object
     */
    private _toCollectionPage;
    /**
     * @private
     * @param {object} error Network error object
     * @returns {Promise<Error>} Promise that rejects with a human-readable error
     */
    private _handleNetworkError;
}
