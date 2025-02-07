import { AxiosRequestConfig, AxiosResponse } from 'feaxios';
import { CancelToken, HttpClient, HttpClientRequestConfig, HttpClientResponse } from './types';
export interface HttpResponse<T = any> extends AxiosResponse<T> {
}
export interface FetchClientConfig<T = any> extends AxiosRequestConfig {
    adapter?: (config: HttpClientRequestConfig) => Promise<HttpClientResponse<T>>;
    cancelToken?: CancelToken;
}
declare function createFetchClient(fetchConfig?: HttpClientRequestConfig): HttpClient;
export declare const fetchClient: HttpClient;
export { createFetchClient as create };
