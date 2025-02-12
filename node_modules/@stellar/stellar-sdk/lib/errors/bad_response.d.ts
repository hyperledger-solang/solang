import { NetworkError } from "./network";
/**
 * BadResponseError is raised when a response from a
 * {@link module:Horizon | Horizon} or {@link module:Federation | Federation}
 * server is invalid in some way. For example, a federation response may exceed
 * the maximum allowed size, or a transaction submission may have failed with
 * Horizon.
 * @augments NetworkError
 * @inheritdoc
 * @category Errors
 *
 * @param {string} message Human-readable error message.
 * @param {any} response Response details, received from the server.
 */
export declare class BadResponseError extends NetworkError {
    constructor(message: string, response: any);
}
