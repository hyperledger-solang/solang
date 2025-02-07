import { NetworkError } from "./network";
/**
 * NotFoundError is raised when the resource requested from Horizon is
 * unavailable.
 * @augments NetworkError
 * @inheritdoc
 * @category Errors
 *
 * @param {string} message Human-readable error message
 * @param {any} response Response details, received from the Horizon server
 */
export declare class NotFoundError extends NetworkError {
    constructor(message: string, response: any);
}
