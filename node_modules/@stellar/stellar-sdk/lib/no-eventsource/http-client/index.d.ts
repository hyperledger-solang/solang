import { HttpClient, HttpClientRequestConfig } from "./types";
declare let httpClient: HttpClient;
declare let create: (config?: HttpClientRequestConfig) => HttpClient;
export { httpClient, create };
export * from "./types";
