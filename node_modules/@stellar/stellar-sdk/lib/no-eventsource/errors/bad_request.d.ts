import { NetworkError } from "./network";
/**
 * BadRequestError is raised when a request made to Horizon is invalid in some
 * way (incorrect timebounds for trade call builders, for example.)
 * @augments NetworkError
 * @inheritdoc
 * @category Errors
 *
 * @param {string} message Human-readable error message
 * @param {any} response Response details, received from the Horizon server
 */
export declare class BadRequestError extends NetworkError {
    constructor(message: string, response: any);
}
