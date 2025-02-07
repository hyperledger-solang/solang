export type Id = string | number;
export interface Request<T> {
    jsonrpc: "2.0";
    id: Id;
    method: string;
    params: T;
}
export interface Notification<T> {
    jsonrpc: "2.0";
    method: string;
    params?: T;
}
export type Response<T, E = any> = {
    jsonrpc: "2.0";
    id: Id;
} & ({
    error: Error<E>;
} | {
    result: T;
});
export interface Error<E = any> {
    code: number;
    message?: string;
    data?: E;
}
/**
 * Sends the jsonrpc 'params' as a single 'param' object (no array support).
 *
 * @param {string} url URL to the RPC instance
 * @param {string} method RPC method name that should be called
 * @param {(any | null)} [param=null] params that should be supplied to the method
 * @returns {Promise<T>}
 * @private
 */
export declare function postObject<T>(url: string, method: string, param?: any): Promise<T>;
