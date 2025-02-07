/**
 * InvalidChallengeError is raised when a challenge transaction does not meet
 * the requirements for a SEP-10 challenge transaction (for example, a non-zero
 * sequence number).
 * @memberof module:WebAuth
 * @category Errors
 *
 * @param {string} message Human-readable error message.
 */
export declare class InvalidChallengeError extends Error {
    __proto__: InvalidChallengeError;
    constructor(message: string);
}
