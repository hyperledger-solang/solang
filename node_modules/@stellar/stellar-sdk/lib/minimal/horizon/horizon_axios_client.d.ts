export declare const version: string;
export interface ServerTime {
    serverTime: number;
    localTimeRecorded: number;
}
/**
 * keep a local map of server times
 * (export this purely for testing purposes)
 *
 * each entry will map the server domain to the last-known time and the local
 * time it was recorded, ex:
 *
 * @example
 * "horizon-testnet.stellar.org": {
 *   serverTime: 1552513039,
 *   localTimeRecorded: 1552513052
 * }
 *
 * @constant {Record.<string, ServerTime>}
 * @default {}
 * @memberof module:Horizon
 */
export declare const SERVER_TIME_MAP: Record<string, ServerTime>;
export declare const AxiosClient: import("../http-client").HttpClient;
export default AxiosClient;
/**
 * Given a hostname, get the current time of that server (i.e., use the last-
 * recorded server time and offset it by the time since then.) If there IS no
 * recorded server time, or it's been 5 minutes since the last, return null.
 * @memberof module:Horizon
 *
 * @param {string} hostname Hostname of a Horizon server.
 * @returns {number} The UNIX timestamp (in seconds, not milliseconds)
 * representing the current time on that server, or `null` if we don't have
 * a record of that time.
 */
export declare function getCurrentServerTime(hostname: string): number | null;
