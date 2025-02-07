/**
 * Global config parameters.
 */
export interface Configuration {
    /**
     * Allow connecting to http servers. This must be set to false in production deployments!
     * @default false
     */
    allowHttp: boolean;
    /**
     * Allow a timeout. Allows user to avoid nasty lag due network issues.
     * @default 0
     */
    timeout: number;
}
/**
 * Global config class.
 *
 * @hideconstructor
 *
 * @example <caption>Usage in node</caption>
 * import { Config } from '@stellar/stellar-sdk';
 * Config.setAllowHttp(true);
 * Config.setTimeout(5000);
 *
 * @example <caption>Usage in the browser</caption>
 * StellarSdk.Config.setAllowHttp(true);
 * StellarSdk.Config.setTimeout(5000);
 */
declare class Config {
    /**
     * Sets `allowHttp` flag globally. When set to `true`, connections to insecure
     * http protocol servers will be allowed. Must be set to `false` in
     * production.
     * @default false
     * @static
     */
    static setAllowHttp(value: boolean): void;
    /**
     * Sets `timeout` flag globally. When set to anything besides 0, the request
     * will timeout after specified time (ms).
     * @default 0
     * @static
     */
    static setTimeout(value: number): void;
    /**
     * Returns the configured `allowHttp` flag.
     * @static
     * @returns {boolean}
     */
    static isAllowHttp(): boolean;
    /**
     * Returns the configured `timeout` flag.
     * @static
     * @returns {number}
     */
    static getTimeout(): number;
    /**
     * Sets all global config flags to default values.
     * @static
     */
    static setDefault(): void;
}
export { Config };
