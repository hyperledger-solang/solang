export declare namespace Api {
    /**
     * Record returned from a federation server.
     */
    interface Record {
        /**
         * The Stellar public key resolved from the federation lookup
         */
        account_id: string;
        /**
         * The type of memo, if any, required to send payments to this user
         */
        memo_type?: string;
        /**
         * The memo value, if any, required to send payments to this user
         */
        memo?: string;
    }
    /**
     * Options for configuring connections to federation servers. You can also use {@link Config} class to set this globally.
     */
    interface Options {
        /**
         * Allow connecting to http servers, default: `false`. This must be set to false in production deployments!
         */
        allowHttp?: boolean;
        /**
         * Allow a timeout, default: 0. Allows user to avoid nasty lag due to TOML resolve issue.
         */
        timeout?: number;
    }
}
