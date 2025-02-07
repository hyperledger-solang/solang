import { HorizonApi } from "../horizon/horizon_api";
/**
 * NetworkError is raised when an interaction with a Horizon server has caused
 * some kind of problem.
 * @category Errors
 *
 * @param {string} message Human-readable error message
 * @param {any} response Response details, received from the Horizon server.
 * @param {HorizonApi.ErrorResponseData} [response.data] The data returned by Horizon as part of the error: {@link https://developers.stellar.org/docs/data/horizon/api-reference/errors/response | Error Response}
 * @param {number} [response.status] HTTP status code describing the basic issue with a submitted transaction {@link https://developers.stellar.org/docs/data/horizon/api-reference/errors/http-status-codes/standard | Standard Status Codes}
 * @param {string} [response.statusText] A human-readable description of what the status code means: {@link https://developers.stellar.org/docs/data/horizon/api-reference/errors/http-status-codes/horizon-specific | Horizon-Specific Status Codes}
 * @param {string} [response.url] URL which can provide more information about the problem that occurred.
 */
export declare class NetworkError extends Error {
    response: {
        data?: HorizonApi.ErrorResponseData;
        status?: number;
        statusText?: string;
        url?: string;
    };
    __proto__: NetworkError;
    constructor(message: string, response: any);
    /**
     * Returns the error response sent by the Horizon server.
     * @returns {any}
     */
    getResponse(): {
        data?: HorizonApi.ErrorResponseData;
        status?: number;
        statusText?: string;
        url?: string;
    };
}
