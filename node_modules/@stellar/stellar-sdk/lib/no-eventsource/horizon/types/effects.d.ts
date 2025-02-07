import { HorizonApi } from "../horizon_api";
import { OfferAsset } from "./offer";
export declare enum EffectType {
    account_created = 0,
    account_removed = 1,
    account_credited = 2,
    account_debited = 3,
    account_thresholds_updated = 4,
    account_home_domain_updated = 5,
    account_flags_updated = 6,
    account_inflation_destination_updated = 7,
    signer_created = 10,
    signer_removed = 11,
    signer_updated = 12,
    trustline_created = 20,
    trustline_removed = 21,
    trustline_updated = 22,
    trustline_authorized = 23,
    trustline_deauthorized = 24,
    trustline_authorized_to_maintain_liabilities = 25,// deprecated, use trustline_flags_updated
    trustline_flags_updated = 26,
    offer_created = 30,
    offer_removed = 31,
    offer_updated = 32,
    trade = 33,
    data_created = 40,
    data_removed = 41,
    data_updated = 42,
    sequence_bumped = 43,
    claimable_balance_created = 50,
    claimable_balance_claimant_created = 51,
    claimable_balance_claimed = 52,
    account_sponsorship_created = 60,
    account_sponsorship_updated = 61,
    account_sponsorship_removed = 62,
    trustline_sponsorship_created = 63,
    trustline_sponsorship_updated = 64,
    trustline_sponsorship_removed = 65,
    data_sponsorship_created = 66,
    data_sponsorship_updated = 67,
    data_sponsorship_removed = 68,
    claimable_balance_sponsorship_created = 69,
    claimable_balance_sponsorship_updated = 70,
    claimable_balance_sponsorship_removed = 71,
    signer_sponsorship_created = 72,
    signer_sponsorship_updated = 73,
    signer_sponsorship_removed = 74,
    claimable_balance_clawed_back = 80,
    liquidity_pool_deposited = 90,
    liquidity_pool_withdrew = 91,
    liquidity_pool_trade = 92,
    liquidity_pool_created = 93,
    liquidity_pool_removed = 94,
    liquidity_pool_revoked = 95,
    contract_credited = 96,
    contract_debited = 97
}
export interface BaseEffectRecord<T extends string = string> extends HorizonApi.BaseResponse {
    id: string;
    account: string;
    paging_token: string;
    type_i: EffectType;
    type: T;
    created_at: string;
}
export interface AccountCreated extends BaseEffectRecord<'account_created'> {
    type_i: EffectType.account_created;
    starting_balance: string;
}
export interface AccountCredited extends BaseEffectRecord<'account_credited'>, OfferAsset {
    type_i: EffectType.account_credited;
    amount: string;
}
export interface AccountDebited extends BaseEffectRecord<'account_debited'>, OfferAsset {
    type_i: EffectType.account_debited;
    amount: string;
}
export interface AccountThresholdsUpdated extends BaseEffectRecord<'account_thresholds_updated'> {
    type_i: EffectType.account_thresholds_updated;
    low_threshold: number;
    med_threshold: number;
    high_threshold: number;
}
export interface AccountHomeDomainUpdated extends BaseEffectRecord<'account_home_domain_updated'> {
    type_i: EffectType.account_home_domain_updated;
    home_domain: string;
}
export interface AccountFlagsUpdated extends BaseEffectRecord<'account_flags_updated'> {
    type_i: EffectType.account_flags_updated;
    auth_required_flag: boolean;
    auth_revokable_flag: boolean;
}
interface DataEvents<T extends string> extends BaseEffectRecord<T> {
    name: boolean;
    value: boolean;
}
export interface DataCreated extends DataEvents<'data_created'> {
    type_i: EffectType.data_created;
}
export interface DataUpdated extends DataEvents<'data_updated'> {
    type_i: EffectType.data_updated;
}
export interface DataRemoved extends DataEvents<'data_removed'> {
    type_i: EffectType.data_removed;
}
export interface SequenceBumped extends BaseEffectRecord<'sequence_bumped'> {
    type_i: EffectType.sequence_bumped;
    new_seq: number | string;
}
interface SignerEvents<T extends string> extends BaseEffectRecord<T> {
    weight: number;
    key: string;
    public_key: string;
}
export interface SignerCreated extends SignerEvents<'signer_created'> {
    type_i: EffectType.signer_created;
}
export interface SignerRemoved extends SignerEvents<'signer_removed'> {
    type_i: EffectType.signer_removed;
}
export interface SignerUpdated extends SignerEvents<'signer_updated'> {
    type_i: EffectType.signer_updated;
}
interface TrustlineEvents<T extends string> extends BaseEffectRecord<T>, OfferAsset {
    limit: string;
    liquidity_pool_id?: string;
}
export interface TrustlineCreated extends TrustlineEvents<'trustline_created'> {
    type_i: EffectType.trustline_created;
}
export interface TrustlineRemoved extends TrustlineEvents<'trustline_removed'> {
    type_i: EffectType.trustline_removed;
}
export interface TrustlineUpdated extends TrustlineEvents<'trustline_updated'> {
    type_i: EffectType.trustline_updated;
}
export interface TrustlineAuthorized extends BaseEffectRecord<'trustline_authorized'> {
    type_i: EffectType.trustline_authorized;
    asset_type: OfferAsset["asset_type"];
    asset_code: OfferAsset["asset_code"];
    trustor: string;
}
export interface TrustlineDeauthorized extends Omit<TrustlineAuthorized, "type_i"> {
    type_i: EffectType.trustline_deauthorized;
}
export interface TrustlineAuthorizedToMaintainLiabilities extends Omit<TrustlineAuthorized, "type_i"> {
    type_i: EffectType.trustline_authorized_to_maintain_liabilities;
}
export interface ClaimableBalanceCreated extends BaseEffectRecord<'claimable_balance_created'> {
    type_i: EffectType.claimable_balance_created;
    amount: string;
    balance_type_i: string;
    asset: string;
}
export interface ClaimableBalanceClaimed extends Omit<ClaimableBalanceCreated, "type_i"> {
    type_i: EffectType.claimable_balance_claimed;
}
export interface ClaimableBalanceClaimantCreated extends Omit<ClaimableBalanceCreated, "type_i"> {
    type_i: EffectType.claimable_balance_claimant_created;
}
interface SponsershipFields {
    sponsor: string;
    new_sponsor: string;
    former_sponsor: string;
}
interface AccountSponsorshipEvents<T extends string> extends BaseEffectRecord<T>, SponsershipFields {
}
export type AccountSponsorshipCreated = Omit<AccountSponsorshipEvents<'account_sponsorship_created'>, "new_sponsor" | "former_sponsor"> & {
    type_i: EffectType.account_sponsorship_created;
};
export type AccountSponsorshipUpdated = Omit<AccountSponsorshipEvents<'account_sponsorship_updated'>, "sponsor"> & {
    type_i: EffectType.account_sponsorship_updated;
};
export type AccountSponsorshipRemoved = Omit<AccountSponsorshipEvents<'account_sponsorship_removed'>, "new_sponsor" | "sponsor"> & {
    type_i: EffectType.account_sponsorship_removed;
};
interface TrustlineSponsorshipEvents<T extends string> extends BaseEffectRecord<T>, SponsershipFields {
    asset?: string;
    liquidity_pool_id?: string;
}
export type TrustlineSponsorshipCreated = Omit<TrustlineSponsorshipEvents<'trustline_sponsorship_created'>, "new_sponsor" | "former_sponsor"> & {
    type_i: EffectType.trustline_sponsorship_created;
};
export type TrustlineSponsorshipUpdated = Omit<TrustlineSponsorshipEvents<'trustline_sponsorship_updated'>, "sponsor"> & {
    type_i: EffectType.trustline_sponsorship_updated;
};
export type TrustlineSponsorshipRemoved = Omit<TrustlineSponsorshipEvents<'trustline_sponsorship_removed'>, "new_sponsor" | "sponsor"> & {
    type_i: EffectType.trustline_sponsorship_removed;
};
interface DataSponsorshipEvents<T extends string> extends BaseEffectRecord<T>, SponsershipFields {
    data_name: string;
}
export type DateSponsorshipCreated = Omit<DataSponsorshipEvents<'data_sponsorship_created'>, "new_sponsor" | "former_sponsor"> & {
    type_i: EffectType.data_sponsorship_created;
};
export type DateSponsorshipUpdated = Omit<DataSponsorshipEvents<'data_sponsorship_updated'>, "sponsor"> & {
    type_i: EffectType.data_sponsorship_updated;
};
export type DateSponsorshipRemoved = Omit<DataSponsorshipEvents<'data_sponsorship_removed'>, "new_sponsor" | "sponsor"> & {
    type_i: EffectType.data_sponsorship_removed;
};
interface ClaimableBalanceSponsorshipEvents<T extends string> extends BaseEffectRecord<T>, SponsershipFields {
    balance_type_i: string;
}
export type ClaimableBalanceSponsorshipCreated = Omit<ClaimableBalanceSponsorshipEvents<'claimable_balance_sponsorship_created'>, "new_sponsor" | "former_sponsor"> & {
    type_i: EffectType.claimable_balance_sponsorship_created;
};
export type ClaimableBalanceSponsorshipUpdated = Omit<ClaimableBalanceSponsorshipEvents<'claimable_balance_sponsorship_updated'>, "sponsor"> & {
    type_i: EffectType.claimable_balance_sponsorship_updated;
};
export type ClaimableBalanceSponsorshipRemoved = Omit<ClaimableBalanceSponsorshipEvents<'claimable_balance_sponsorship_removed'>, "new_sponsor" | "sponsor"> & {
    type_i: EffectType.claimable_balance_sponsorship_removed;
};
interface SignerSponsorshipEvents<T extends string> extends BaseEffectRecord<T>, SponsershipFields {
    signer: string;
}
export type SignerSponsorshipCreated = Omit<SignerSponsorshipEvents<'signer_sponsorship_created'>, "new_sponsor" | "former_sponsor"> & {
    type_i: EffectType.signer_sponsorship_created;
};
export type SignerSponsorshipUpdated = Omit<SignerSponsorshipEvents<'signer_sponsorship_updated'>, "sponsor"> & {
    type_i: EffectType.signer_sponsorship_updated;
};
export type SignerSponsorshipRemoved = Omit<SignerSponsorshipEvents<'signer_sponsorship_removed'>, "new_sponsor" | "sponsor"> & {
    type_i: EffectType.signer_sponsorship_removed;
};
export interface ClaimableBalanceClawedBack extends HorizonApi.BaseResponse {
    balance_id: string;
}
export interface LiquidityPoolEffectRecord extends HorizonApi.BaseResponse {
    id: string;
    fee_bp: number;
    type: HorizonApi.LiquidityPoolType;
    total_trustlines: string;
    total_shares: string;
    reserves: HorizonApi.Reserve[];
}
export interface LiquidityPoolDeposited extends BaseEffectRecord<'liquidity_pool_deposited'> {
    type_i: EffectType.liquidity_pool_deposited;
    liquidity_pool: LiquidityPoolEffectRecord;
    reserves_deposited: HorizonApi.Reserve[];
    shares_received: string;
}
export interface LiquidityPoolWithdrew extends BaseEffectRecord<'liquidity_pool_withdrew'> {
    type_i: EffectType.liquidity_pool_withdrew;
    liquidity_pool: LiquidityPoolEffectRecord;
    reserves_received: HorizonApi.Reserve[];
    shares_redeemed: string;
}
export interface LiquidityPoolTrade extends BaseEffectRecord<'liquidity_pool_trade'> {
    type_i: EffectType.liquidity_pool_trade;
    liquidity_pool: LiquidityPoolEffectRecord;
    sold: HorizonApi.Reserve;
    bought: HorizonApi.Reserve;
}
export interface LiquidityPoolCreated extends BaseEffectRecord<'liquidity_pool_created'> {
    type_i: EffectType.liquidity_pool_created;
    liquidity_pool: LiquidityPoolEffectRecord;
}
export interface LiquidityPoolRemoved extends BaseEffectRecord<'liquidity_pool_removed'> {
    type_i: EffectType.liquidity_pool_removed;
    liquidity_pool_id: string;
}
export interface LiquidityPoolRevoked extends BaseEffectRecord<'liquidity_pool_revoked'> {
    type_i: EffectType.liquidity_pool_revoked;
    liquidity_pool: LiquidityPoolEffectRecord;
    reserves_revoked: [
        {
            asset: string;
            amount: string;
            claimable_balance_id: string;
        }
    ];
    shares_revoked: string;
}
export interface ContractCredited extends BaseEffectRecord<'contract_credited'>, OfferAsset {
    type_i: EffectType.contract_credited;
    contract: string;
    amount: string;
}
export interface ContractDebited extends BaseEffectRecord<'contract_debited'>, OfferAsset {
    type_i: EffectType.contract_debited;
    contract: string;
    amount: string;
}
export {};
