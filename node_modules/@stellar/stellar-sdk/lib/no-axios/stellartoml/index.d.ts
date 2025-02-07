import { Networks } from "@stellar/stellar-base";
/** @module StellarToml */
/**
 * The maximum size of stellar.toml file, in bytes
 * @constant {number}
 * @default 102400
 */
export declare const STELLAR_TOML_MAX_SIZE: number;
/**
 * Resolver allows resolving `stellar.toml` files.
 * @memberof module:StellarToml
 * @hideconstructor
 */
export declare class Resolver {
    /**
     * Returns a parsed `stellar.toml` file for a given domain.
     * @see {@link https://developers.stellar.org/docs/tokens/publishing-asset-info | Stellar.toml doc}
     *
     * @param {string} domain Domain to get stellar.toml file for
     * @param {object} [opts] Options object
     * @param {boolean} [opts.allowHttp=false] - Allow connecting to http servers. This must be set to false in production deployments!
     * @param {number} [opts.timeout=0] - Allow a timeout. Allows user to avoid nasty lag due to TOML resolve issue.
     * @returns {Promise} A `Promise` that resolves to the parsed stellar.toml object
     *
     * @example
     * StellarSdk.StellarToml.Resolver.resolve('acme.com')
     *   .then(stellarToml => {
     *     // stellarToml in an object representing domain stellar.toml file.
     *   })
     *   .catch(error => {
     *     // stellar.toml does not exist or is invalid
     *   });
     */
    static resolve(domain: string, opts?: Api.StellarTomlResolveOptions): Promise<Api.StellarToml>;
}
export declare namespace Api {
    interface StellarTomlResolveOptions {
        allowHttp?: boolean;
        timeout?: number;
        allowedRedirects?: number;
    }
    type Url = string;
    type PublicKey = string;
    type ISODateTime = string;
    interface Documentation {
        ORG_NAME?: string;
        ORG_DBA?: string;
        ORG_URL?: Url;
        ORG_PHONE_NUMBER?: string;
        ORG_LOGO?: Url;
        ORG_LICENSE_NUMBER?: string;
        ORG_LICENSING_AUTHORITY?: string;
        ORG_LICENSE_TYPE?: string;
        ORG_DESCRIPTION?: string;
        ORG_PHYSICAL_ADDRESS?: string;
        ORG_PHYSICAL_ADDRESS_ATTESTATION?: string;
        ORG_PHONE_NUMBER_ATTESTATION?: string;
        ORG_OFFICIAL_EMAIL?: string;
        ORG_SUPPORT_EMAIL?: string;
        ORG_KEYBASE?: string;
        ORG_TWITTER?: string;
        ORG_GITHUB?: string;
        [key: string]: unknown;
    }
    interface Principal {
        name: string;
        email: string;
        github?: string;
        keybase?: string;
        telegram?: string;
        twitter?: string;
        id_photo_hash?: string;
        verification_photo_hash?: string;
        [key: string]: unknown;
    }
    interface Currency {
        code?: string;
        code_template?: string;
        issuer?: PublicKey;
        display_decimals?: number;
        status?: "live" | "dead" | "test" | "private";
        name?: string;
        desc?: string;
        conditions?: string;
        fixed_number?: number;
        max_number?: number;
        is_asset_anchored?: boolean;
        anchor_asset_type?: "fiat" | "crypto" | "nft" | "stock" | "bond" | "commodity" | "realestate" | "other";
        anchor_asset?: string;
        attestation_of_reserve?: Url;
        attestation_of_reserve_amount?: string;
        attestation_of_reserve_last_audit?: ISODateTime;
        is_unlimited?: boolean;
        redemption_instructions?: string;
        image?: Url;
        regulated?: boolean;
        collateral_addresses?: string[];
        collateral_address_messages?: string[];
        collateral_address_signatures?: string[];
        approval_server?: Url;
        approval_criteria?: string;
        [key: string]: unknown;
    }
    interface Validator {
        ALIAS?: string;
        DISPLAY_NAME?: string;
        PUBLIC_KEY?: PublicKey;
        HOST?: string;
        HISTORY?: Url;
        [key: string]: unknown;
    }
    interface StellarToml {
        VERSION?: string;
        ACCOUNTS?: PublicKey[];
        NETWORK_PASSPHRASE?: Networks;
        TRANSFER_SERVER_SEP0024?: Url;
        TRANSFER_SERVER?: Url;
        KYC_SERVER?: Url;
        WEB_AUTH_ENDPOINT?: Url;
        FEDERATION_SERVER?: Url;
        SIGNING_KEY?: PublicKey;
        HORIZON_URL?: Url;
        URI_REQUEST_SIGNING_KEY?: PublicKey;
        DIRECT_PAYMENT_SERVER?: Url;
        ANCHOR_QUOTE_SERVER?: Url;
        DOCUMENTATION?: Documentation;
        PRINCIPALS?: Principal[];
        CURRENCIES?: Currency[];
        VALIDATORS?: Validator[];
        [key: string]: unknown;
    }
}
