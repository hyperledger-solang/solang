interface AxiosRetryConfig {
    retries?: number;
    shouldResetTimeout?: boolean;
    retryCondition?: (error: AxiosError) => boolean | Promise<boolean>;
    retryDelay?: (retryCount: number, error: AxiosError) => number;
    onRetry?: (retryCount: number, error: AxiosError, requestConfig: AxiosRequestConfig) => Promise<void> | void;
}
interface AxiosRetryConfigExtended extends AxiosRetryConfig {
    retryCount?: number;
    lastRequestTime?: number;
}
interface AxiosRetryReturn {
    requestInterceptorId: number;
    responseInterceptorId: number;
}
interface AxiosRetry {
    (axiosInstance: AxiosStatic | AxiosInstance, axiosRetryConfig?: AxiosRetryConfig): AxiosRetryReturn;
    isNetworkError(error: AxiosError): boolean;
    isRetryableError(error: AxiosError): boolean;
    isSafeRequestError(error: AxiosError): boolean;
    isIdempotentRequestError(error: AxiosError): boolean;
    isNetworkOrIdempotentRequestError(error: AxiosError): boolean;
    exponentialDelay(retryNumber?: number, error?: AxiosError, delayFactor?: number): number;
}
declare function isNetworkError(error: AxiosError): boolean;
declare function isRetryableError(error: AxiosError): boolean;
declare function isSafeRequestError(error: AxiosError): boolean;
declare function isIdempotentRequestError(error: AxiosError): boolean;
declare function isNetworkOrIdempotentRequestError(error: AxiosError): boolean;
declare function exponentialDelay(retryNumber?: number, _error?: AxiosError | undefined, delayFactor?: number): number;
declare const DEFAULT_OPTIONS: Required<AxiosRetryConfig>;
declare const axiosRetry: AxiosRetry;

type AxiosRequestTransformer = (this: InternalAxiosRequestConfig, data: any, headers: Headers) => any;
type AxiosResponseTransformer = (this: InternalAxiosRequestConfig, data: any, headers: HeadersInit, status?: number) => any;
type ResponseType = "arrayBuffer" | "blob" | "json" | "text" | "stream";
type Method = 'get' | 'GET' | 'delete' | 'DELETE' | 'head' | 'HEAD' | 'options' | 'OPTIONS' | 'post' | 'POST' | 'put' | 'PUT' | 'patch' | 'PATCH' | 'purge' | 'PURGE' | 'link' | 'LINK' | 'unlink' | 'UNLINK';
interface FormDataVisitorHelpers {
    defaultVisitor: SerializerVisitor;
    convertValue: (value: any) => any;
    isVisitable: (value: any) => boolean;
}
type SerializerVisitor = (this: GenericFormData, value: any, key: string | number, path: null | Array<string | number>, helpers: FormDataVisitorHelpers) => boolean;
interface GenericFormData {
    append(name: string, value: any, options?: any): any;
}
interface SerializerOptions {
    visitor?: SerializerVisitor;
    dots?: boolean;
    metaTokens?: boolean;
    indexes?: boolean | null;
}
type ParamEncoder = (value: any, defaultEncoder: (value: any) => any) => any;
type CustomParamsSerializer = (params: Record<string, any>, options?: ParamsSerializerOptions) => string;
interface ParamsSerializerOptions extends SerializerOptions {
    encode?: ParamEncoder;
    serialize?: CustomParamsSerializer;
}
interface AxiosRequestConfig<D = any> {
    url?: string;
    method?: Method | string;
    baseURL?: string;
    transformRequest?: AxiosRequestTransformer | AxiosRequestTransformer[];
    transformResponse?: AxiosResponseTransformer | AxiosResponseTransformer[];
    headers?: HeadersInit;
    params?: Record<string, any>;
    paramsSerializer?: CustomParamsSerializer;
    data?: D;
    timeout?: number;
    timeoutErrorMessage?: string;
    withCredentials?: boolean;
    responseType?: ResponseType;
    validateStatus?: ((status: number) => boolean) | null;
    signal?: AbortSignal;
    fetchOptions?: RequestInit;
    retry?: AxiosRetryConfigExtended;
}
type RawAxiosRequestConfig<D = any> = AxiosRequestConfig<D>;
interface InternalAxiosRequestConfig<D = any> extends Omit<AxiosRequestConfig<D>, "headers"> {
    headers: Headers;
}
interface AxiosDefaults<D = any> extends Omit<AxiosRequestConfig<D>, "headers"> {
    headers: HeadersInit;
}
interface CreateAxiosDefaults<D = any> extends Omit<AxiosRequestConfig<D>, "headers"> {
    headers?: HeadersInit;
}
interface AxiosResponse<T = any, D = any> {
    data: T;
    status: number;
    statusText: string;
    headers: Headers;
    config: InternalAxiosRequestConfig<D>;
    request?: Request;
}
type AxiosPromise<T = any> = Promise<AxiosResponse<T>>;
interface AxiosInterceptorOptions {
    runWhen?: (config: InternalAxiosRequestConfig) => boolean;
}
type FulfillCallback<V> = ((value: V) => V | Promise<V>) | null;
type RejectCallback = ((error: any) => any) | null;
interface AxiosInterceptorManager<V> {
    use(onFulfilled?: FulfillCallback<V>, onRejected?: RejectCallback, options?: AxiosInterceptorOptions): number;
    eject(id: number): void;
    clear(): void;
}
type AxiosInterceptor<V> = {
    fulfilled?: FulfillCallback<V>;
    rejected?: RejectCallback;
    synchronous?: boolean;
    runWhen?: (config: InternalAxiosRequestConfig) => boolean;
};
interface AxiosInstance {
    defaults: CreateAxiosDefaults;
    interceptors: {
        request: AxiosInterceptorManager<InternalAxiosRequestConfig>;
        response: AxiosInterceptorManager<AxiosResponse>;
    };
    getUri: (config?: AxiosRequestConfig) => string;
    request: <T = any, R = AxiosResponse<T, any>, D = any>(config: AxiosRequestConfig<D>) => Promise<R>;
    get: <T = any, R = AxiosResponse<T, any>, D = any>(url: string, config?: AxiosRequestConfig<D> | undefined) => Promise<R>;
    delete: <T = any, R = AxiosResponse<T, any>, D = any>(url: string, config?: AxiosRequestConfig<D> | undefined) => Promise<R>;
    head: <T = any, R = AxiosResponse<T, any>, D = any>(url: string, config?: AxiosRequestConfig<D> | undefined) => Promise<R>;
    options: <T = any, R = AxiosResponse<T, any>, D = any>(url: string, config?: AxiosRequestConfig<D> | undefined) => Promise<R>;
    post: <T = any, R = AxiosResponse<T, any>, D = any>(url: string, data?: D | undefined, config?: AxiosRequestConfig<D> | undefined) => Promise<R>;
    put: <T = any, R = AxiosResponse<T, any>, D = any>(url: string, data?: D | undefined, config?: AxiosRequestConfig<D> | undefined) => Promise<R>;
    patch: <T = any, R = AxiosResponse<T, any>, D = any>(url: string, data?: D | undefined, config?: AxiosRequestConfig<D> | undefined) => Promise<R>;
    postForm: <T = any, R = AxiosResponse<T, any>, D = any>(url: string, data?: D | undefined, config?: AxiosRequestConfig<D> | undefined) => Promise<R>;
    putForm: <T = any, R = AxiosResponse<T, any>, D = any>(url: string, data?: D | undefined, config?: AxiosRequestConfig<D> | undefined) => Promise<R>;
    patchForm: <T = any, R = AxiosResponse<T, any>, D = any>(url: string, data?: D | undefined, config?: AxiosRequestConfig<D> | undefined) => Promise<R>;
    <T = any, R = AxiosResponse<T>, D = any>(config: AxiosRequestConfig<D>): Promise<R>;
    <T = any, R = AxiosResponse<T>, D = any>(url: string, config?: AxiosRequestConfig<D>): Promise<R>;
}
interface AxiosStatic extends AxiosInstance {
    create: (defaults?: CreateAxiosDefaults) => AxiosInstance;
}

declare class AxiosError<T = unknown, D = any> extends Error {
    config?: InternalAxiosRequestConfig<D>;
    code?: string;
    request?: any;
    response?: AxiosResponse<T, D>;
    status?: number;
    isAxiosError: boolean;
    constructor(message?: string, code?: string, config?: InternalAxiosRequestConfig<D>, request?: any, response?: AxiosResponse<T, D>);
    static readonly ERR_BAD_OPTION_VALUE = "ERR_BAD_OPTION_VALUE";
    static readonly ERR_BAD_OPTION = "ERR_BAD_OPTION";
    static readonly ERR_NETWORK = "ERR_NETWORK";
    static readonly ERR_BAD_RESPONSE = "ERR_BAD_RESPONSE";
    static readonly ERR_BAD_REQUEST = "ERR_BAD_REQUEST";
    static readonly ERR_INVALID_URL = "ERR_INVALID_URL";
    static readonly ERR_CANCELED = "ERR_CANCELED";
    static readonly ECONNABORTED = "ECONNABORTED";
    static readonly ETIMEDOUT = "ETIMEDOUT";
}
declare class CanceledError<T = unknown, D = any> extends AxiosError<T, D> {
    constructor(message: string | null | undefined, config?: InternalAxiosRequestConfig<D>, request?: any);
}
declare function isAxiosError<T = any, D = any>(payload: any): payload is AxiosError<T, D>;
declare const axios: AxiosStatic;

export { AxiosError as A, isRetryableError as B, CanceledError as C, isSafeRequestError as D, isIdempotentRequestError as E, type FormDataVisitorHelpers as F, isNetworkOrIdempotentRequestError as G, exponentialDelay as H, type InternalAxiosRequestConfig as I, DEFAULT_OPTIONS as J, type Method as M, type ParamEncoder as P, type ResponseType as R, type SerializerVisitor as S, axios as a, type AxiosRequestTransformer as b, type AxiosResponseTransformer as c, type SerializerOptions as d, type CustomParamsSerializer as e, type ParamsSerializerOptions as f, type AxiosRequestConfig as g, type RawAxiosRequestConfig as h, isAxiosError as i, type AxiosDefaults as j, type CreateAxiosDefaults as k, type AxiosResponse as l, type AxiosPromise as m, type AxiosInterceptorOptions as n, type FulfillCallback as o, type RejectCallback as p, type AxiosInterceptorManager as q, type AxiosInterceptor as r, type AxiosInstance as s, type AxiosStatic as t, axiosRetry as u, type AxiosRetryConfig as v, type AxiosRetryConfigExtended as w, type AxiosRetryReturn as x, type AxiosRetry as y, isNetworkError as z };
