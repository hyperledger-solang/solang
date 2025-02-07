export type HttpResponseHeaders = Record<string, string | boolean | undefined> & {
    'set-cookie'?: string[];
};
export interface HttpClientDefaults extends Omit<HttpClientRequestConfig, 'headers'> {
    headers?: [string, string][] | Record<string, string> | Headers | undefined;
}
export interface HttpClientResponse<T = any> {
    data: T;
    headers: HttpResponseHeaders;
    config: any;
    status: number;
    statusText: string;
}
export interface CancelToken {
    promise: Promise<void>;
    throwIfRequested(): void;
    reason?: string;
}
type HeadersInit = [string, string][] | Record<string, string> | Headers;
export interface HttpClientRequestConfig<D = any> {
    url?: string;
    method?: string;
    baseURL?: string;
    data?: D;
    timeout?: number;
    fetchOptions?: Record<string, any>;
    headers?: HeadersInit;
    params?: Record<string, any>;
    maxContentLength?: number;
    maxRedirects?: number;
    cancelToken?: CancelToken;
    adapter?: (config: HttpClientRequestConfig) => Promise<HttpClientResponse>;
}
export interface HttpClient {
    get: <T = any>(url: string, config?: HttpClientRequestConfig) => Promise<HttpClientResponse<T>>;
    delete: <T = any>(url: string, config?: HttpClientRequestConfig) => Promise<HttpClientResponse<T>>;
    head: <T = any>(url: string, config?: HttpClientRequestConfig) => Promise<HttpClientResponse<T>>;
    options: <T = any>(url: string, config?: HttpClientRequestConfig) => Promise<HttpClientResponse<T>>;
    post: <T = any>(url: string, data?: any, config?: HttpClientRequestConfig) => Promise<HttpClientResponse<T>>;
    put: <T = any>(url: string, data?: any, config?: HttpClientRequestConfig) => Promise<HttpClientResponse<T>>;
    patch: <T = any>(url: string, data?: any, config?: HttpClientRequestConfig) => Promise<HttpClientResponse<T>>;
    postForm: <T = any>(url: string, data?: any, config?: HttpClientRequestConfig) => Promise<HttpClientResponse<T>>;
    putForm: <T = any>(url: string, data?: any, config?: HttpClientRequestConfig) => Promise<HttpClientResponse<T>>;
    patchForm: <T = any>(url: string, data?: any, config?: HttpClientRequestConfig) => Promise<HttpClientResponse<T>>;
    interceptors: {
        request: InterceptorManager<HttpClientRequestConfig>;
        response: InterceptorManager<HttpClientResponse>;
    };
    defaults: HttpClientDefaults;
    CancelToken: typeof CancelToken;
    isCancel: (value: any) => boolean;
    makeRequest: <T = any>(config: HttpClientRequestConfig) => Promise<HttpClientResponse<T>>;
    create: (config?: HttpClientRequestConfig) => HttpClient;
}
export interface Interceptor<V> {
    fulfilled: (value: V) => V | Promise<V>;
    rejected?: (error: any) => any;
}
export interface InterceptorManager<V> {
    use(fulfilled: (value: V) => V | Promise<V>, rejected?: (error: any) => any): number;
    eject(id: number): void;
    forEach(fn: (interceptor: Interceptor<V>) => void): void;
    handlers: Array<Interceptor<V> | null>;
}
export declare class CancelToken {
    promise: Promise<void>;
    reason?: string;
    constructor(executor: (cancel: (reason?: string) => void) => void);
}
export {};
