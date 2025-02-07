import { Asset } from "@stellar/stellar-base";
import { HorizonApi } from "./horizon_api";
import { AccountRecordSigners as AccountRecordSignersType } from "./types/account";
import { AssetRecord as AssetRecordType } from "./types/assets";
import * as Effects from "./types/effects";
import { OfferRecord as OfferRecordType } from "./types/offer";
import { Trade } from "./types/trade";
export declare namespace ServerApi {
    export type OfferRecord = OfferRecordType;
    export type AccountRecordSigners = AccountRecordSignersType;
    export type AssetRecord = AssetRecordType;
    export interface CollectionPage<T extends HorizonApi.BaseResponse = HorizonApi.BaseResponse> {
        records: T[];
        next: () => Promise<CollectionPage<T>>;
        prev: () => Promise<CollectionPage<T>>;
    }
    export interface CallFunctionTemplateOptions {
        cursor?: string | number;
        limit?: number;
        order?: "asc" | "desc";
    }
    export type CallFunction<T extends HorizonApi.BaseResponse = HorizonApi.BaseResponse> = () => Promise<T>;
    export type CallCollectionFunction<T extends HorizonApi.BaseResponse = HorizonApi.BaseResponse> = (options?: CallFunctionTemplateOptions) => Promise<CollectionPage<T>>;
    type BaseEffectRecordFromTypes = Effects.AccountCreated | Effects.AccountCredited | Effects.AccountDebited | Effects.AccountThresholdsUpdated | Effects.AccountHomeDomainUpdated | Effects.AccountFlagsUpdated | Effects.DataCreated | Effects.DataRemoved | Effects.DataUpdated | Effects.SequenceBumped | Effects.SignerCreated | Effects.SignerRemoved | Effects.SignerUpdated | Effects.TrustlineCreated | Effects.TrustlineRemoved | Effects.TrustlineUpdated | Effects.TrustlineAuthorized | Effects.TrustlineDeauthorized | Effects.TrustlineAuthorizedToMaintainLiabilities | Effects.ClaimableBalanceCreated | Effects.ClaimableBalanceClaimed | Effects.ClaimableBalanceClaimantCreated | Effects.AccountSponsorshipCreated | Effects.AccountSponsorshipRemoved | Effects.AccountSponsorshipUpdated | Effects.TrustlineSponsorshipCreated | Effects.TrustlineSponsorshipUpdated | Effects.TrustlineSponsorshipRemoved | Effects.DateSponsorshipCreated | Effects.DateSponsorshipUpdated | Effects.DateSponsorshipRemoved | Effects.ClaimableBalanceSponsorshipCreated | Effects.ClaimableBalanceSponsorshipRemoved | Effects.ClaimableBalanceSponsorshipUpdated | Effects.SignerSponsorshipCreated | Effects.SignerSponsorshipUpdated | Effects.SignerSponsorshipRemoved | Effects.LiquidityPoolDeposited | Effects.LiquidityPoolWithdrew | Effects.LiquidityPoolCreated | Effects.LiquidityPoolRemoved | Effects.LiquidityPoolRevoked | Effects.LiquidityPoolTrade | Effects.ContractCredited | Effects.ContractDebited | Trade;
    export type EffectRecord = BaseEffectRecordFromTypes & EffectRecordMethods;
    export const EffectType: typeof Effects.EffectType;
    export interface ClaimableBalanceRecord extends HorizonApi.BaseResponse {
        id: string;
        paging_token: string;
        asset: string;
        amount: string;
        sponsor?: string;
        last_modified_ledger: number;
        claimants: HorizonApi.Claimant[];
    }
    export interface AccountRecord extends HorizonApi.BaseResponse {
        id: string;
        paging_token: string;
        account_id: string;
        sequence: string;
        sequence_ledger?: number;
        sequence_time?: string;
        subentry_count: number;
        home_domain?: string;
        inflation_destination?: string;
        last_modified_ledger: number;
        last_modified_time: string;
        thresholds: HorizonApi.AccountThresholds;
        flags: HorizonApi.Flags;
        balances: HorizonApi.BalanceLine[];
        signers: AccountRecordSigners[];
        data: (options: {
            value: string;
        }) => Promise<{
            value: string;
        }>;
        data_attr: {
            [key: string]: string;
        };
        sponsor?: string;
        num_sponsoring: number;
        num_sponsored: number;
        effects: CallCollectionFunction<EffectRecord>;
        offers: CallCollectionFunction<OfferRecord>;
        operations: CallCollectionFunction<OperationRecord>;
        payments: CallCollectionFunction<PaymentOperationRecord>;
        trades: CallCollectionFunction<TradeRecord>;
    }
    export interface LiquidityPoolRecord extends HorizonApi.BaseResponse {
        id: string;
        paging_token: string;
        fee_bp: number;
        type: HorizonApi.LiquidityPoolType;
        total_trustlines: string;
        total_shares: string;
        reserves: HorizonApi.Reserve[];
    }
    export enum TradeType {
        all = "all",
        liquidityPools = "liquidity_pool",
        orderbook = "orderbook"
    }
    interface EffectRecordMethods {
        operation?: CallFunction<OperationRecord>;
        precedes?: CallFunction<EffectRecord>;
        succeeds?: CallFunction<EffectRecord>;
    }
    export interface LedgerRecord extends HorizonApi.BaseResponse {
        id: string;
        paging_token: string;
        hash: string;
        prev_hash: string;
        sequence: number;
        successful_transaction_count: number;
        failed_transaction_count: number;
        operation_count: number;
        tx_set_operation_count: number | null;
        closed_at: string;
        total_coins: string;
        fee_pool: string;
        max_tx_set_size: number;
        protocol_version: number;
        header_xdr: string;
        base_fee_in_stroops: number;
        base_reserve_in_stroops: number;
        effects: CallCollectionFunction<EffectRecord>;
        operations: CallCollectionFunction<OperationRecord>;
        self: CallFunction<LedgerRecord>;
        transactions: CallCollectionFunction<TransactionRecord>;
    }
    import OperationResponseType = HorizonApi.OperationResponseType;
    import OperationResponseTypeI = HorizonApi.OperationResponseTypeI;
    export interface BaseOperationRecord<T extends OperationResponseType = OperationResponseType, TI extends OperationResponseTypeI = OperationResponseTypeI> extends HorizonApi.BaseOperationResponse<T, TI> {
        self: CallFunction<OperationRecord>;
        succeeds: CallFunction<OperationRecord>;
        precedes: CallFunction<OperationRecord>;
        effects: CallCollectionFunction<EffectRecord>;
        transaction: CallFunction<TransactionRecord>;
    }
    export interface CreateAccountOperationRecord extends BaseOperationRecord<OperationResponseType.createAccount, OperationResponseTypeI.createAccount>, HorizonApi.CreateAccountOperationResponse {
    }
    export interface PaymentOperationRecord extends BaseOperationRecord<OperationResponseType.payment, OperationResponseTypeI.payment>, HorizonApi.PaymentOperationResponse {
        sender: CallFunction<AccountRecord>;
        receiver: CallFunction<AccountRecord>;
    }
    export interface PathPaymentOperationRecord extends BaseOperationRecord<OperationResponseType.pathPayment, OperationResponseTypeI.pathPayment>, HorizonApi.PathPaymentOperationResponse {
    }
    export interface PathPaymentStrictSendOperationRecord extends BaseOperationRecord<OperationResponseType.pathPaymentStrictSend, OperationResponseTypeI.pathPaymentStrictSend>, HorizonApi.PathPaymentStrictSendOperationResponse {
    }
    export interface ManageOfferOperationRecord extends BaseOperationRecord<OperationResponseType.manageOffer, OperationResponseTypeI.manageOffer>, HorizonApi.ManageOfferOperationResponse {
    }
    export interface PassiveOfferOperationRecord extends BaseOperationRecord<OperationResponseType.createPassiveOffer, OperationResponseTypeI.createPassiveOffer>, HorizonApi.PassiveOfferOperationResponse {
    }
    export interface SetOptionsOperationRecord extends BaseOperationRecord<OperationResponseType.setOptions, OperationResponseTypeI.setOptions>, HorizonApi.SetOptionsOperationResponse {
    }
    export interface ChangeTrustOperationRecord extends BaseOperationRecord<OperationResponseType.changeTrust, OperationResponseTypeI.changeTrust>, HorizonApi.ChangeTrustOperationResponse {
    }
    export interface AllowTrustOperationRecord extends BaseOperationRecord<OperationResponseType.allowTrust, OperationResponseTypeI.allowTrust>, HorizonApi.AllowTrustOperationResponse {
    }
    export interface AccountMergeOperationRecord extends BaseOperationRecord<OperationResponseType.accountMerge, OperationResponseTypeI.accountMerge>, HorizonApi.AccountMergeOperationResponse {
    }
    export interface InflationOperationRecord extends BaseOperationRecord<OperationResponseType.inflation, OperationResponseTypeI.inflation>, HorizonApi.InflationOperationResponse {
    }
    export interface ManageDataOperationRecord extends BaseOperationRecord<OperationResponseType.manageData, OperationResponseTypeI.manageData>, HorizonApi.ManageDataOperationResponse {
    }
    export interface BumpSequenceOperationRecord extends BaseOperationRecord<OperationResponseType.bumpSequence, OperationResponseTypeI.bumpSequence>, HorizonApi.BumpSequenceOperationResponse {
    }
    export interface CreateClaimableBalanceOperationRecord extends BaseOperationRecord<OperationResponseType.createClaimableBalance, OperationResponseTypeI.createClaimableBalance>, HorizonApi.CreateClaimableBalanceOperationResponse {
    }
    export interface ClaimClaimableBalanceOperationRecord extends BaseOperationRecord<OperationResponseType.claimClaimableBalance, OperationResponseTypeI.claimClaimableBalance>, HorizonApi.ClaimClaimableBalanceOperationResponse {
    }
    export interface BeginSponsoringFutureReservesOperationRecord extends BaseOperationRecord<OperationResponseType.beginSponsoringFutureReserves, OperationResponseTypeI.beginSponsoringFutureReserves>, HorizonApi.BeginSponsoringFutureReservesOperationResponse {
    }
    export interface EndSponsoringFutureReservesOperationRecord extends BaseOperationRecord<OperationResponseType.endSponsoringFutureReserves, OperationResponseTypeI.endSponsoringFutureReserves>, HorizonApi.EndSponsoringFutureReservesOperationResponse {
    }
    export interface RevokeSponsorshipOperationRecord extends BaseOperationRecord<OperationResponseType.revokeSponsorship, OperationResponseTypeI.revokeSponsorship>, HorizonApi.RevokeSponsorshipOperationResponse {
    }
    export interface ClawbackOperationRecord extends BaseOperationRecord<OperationResponseType.clawback, OperationResponseTypeI.clawback>, HorizonApi.ClawbackOperationResponse {
    }
    export interface ClawbackClaimableBalanceOperationRecord extends BaseOperationRecord<OperationResponseType.clawbackClaimableBalance, OperationResponseTypeI.clawbackClaimableBalance>, HorizonApi.ClawbackClaimableBalanceOperationResponse {
    }
    export interface SetTrustLineFlagsOperationRecord extends BaseOperationRecord<OperationResponseType.setTrustLineFlags, OperationResponseTypeI.setTrustLineFlags>, HorizonApi.SetTrustLineFlagsOperationResponse {
    }
    export interface DepositLiquidityOperationRecord extends BaseOperationRecord<OperationResponseType.liquidityPoolDeposit, OperationResponseTypeI.liquidityPoolDeposit>, HorizonApi.DepositLiquidityOperationResponse {
    }
    export interface WithdrawLiquidityOperationRecord extends BaseOperationRecord<OperationResponseType.liquidityPoolWithdraw, OperationResponseTypeI.liquidityPoolWithdraw>, HorizonApi.WithdrawLiquidityOperationResponse {
    }
    export interface InvokeHostFunctionOperationRecord extends BaseOperationRecord<OperationResponseType.invokeHostFunction, OperationResponseTypeI.invokeHostFunction>, HorizonApi.InvokeHostFunctionOperationResponse {
    }
    export interface BumpFootprintExpirationOperationRecord extends BaseOperationRecord<OperationResponseType.bumpFootprintExpiration, OperationResponseTypeI.bumpFootprintExpiration>, HorizonApi.BumpFootprintExpirationOperationResponse {
    }
    export interface RestoreFootprintOperationRecord extends BaseOperationRecord<OperationResponseType.restoreFootprint, OperationResponseTypeI.restoreFootprint>, HorizonApi.RestoreFootprintOperationResponse {
    }
    export type OperationRecord = CreateAccountOperationRecord | PaymentOperationRecord | PathPaymentOperationRecord | ManageOfferOperationRecord | PassiveOfferOperationRecord | SetOptionsOperationRecord | ChangeTrustOperationRecord | AllowTrustOperationRecord | AccountMergeOperationRecord | InflationOperationRecord | ManageDataOperationRecord | BumpSequenceOperationRecord | PathPaymentStrictSendOperationRecord | CreateClaimableBalanceOperationRecord | ClaimClaimableBalanceOperationRecord | BeginSponsoringFutureReservesOperationRecord | EndSponsoringFutureReservesOperationRecord | RevokeSponsorshipOperationRecord | ClawbackClaimableBalanceOperationRecord | ClawbackOperationRecord | SetTrustLineFlagsOperationRecord | DepositLiquidityOperationRecord | WithdrawLiquidityOperationRecord | InvokeHostFunctionOperationRecord | BumpFootprintExpirationOperationRecord | RestoreFootprintOperationRecord;
    export namespace TradeRecord {
        interface Base extends HorizonApi.BaseResponse {
            id: string;
            paging_token: string;
            ledger_close_time: string;
            trade_type: TradeType;
            base_account?: string;
            base_amount: string;
            base_asset_type: string;
            base_asset_code?: string;
            base_asset_issuer?: string;
            counter_account?: string;
            counter_amount: string;
            counter_asset_type: string;
            counter_asset_code?: string;
            counter_asset_issuer?: string;
            base_is_seller: boolean;
            price?: {
                n: string;
                d: string;
            };
            operation: CallFunction<OperationRecord>;
        }
        export interface Orderbook extends Base {
            trade_type: TradeType.orderbook;
            base_offer_id: string;
            base_account: string;
            counter_offer_id: string;
            counter_account: string;
            base: CallFunction<AccountRecord>;
            counter: CallFunction<AccountRecord>;
        }
        export interface LiquidityPool extends Base {
            trade_type: TradeType.liquidityPools;
            base_liquidity_pool_id?: string;
            counter_liquidity_pool_id?: string;
            liquidity_pool_fee_bp: number;
            base: CallFunction<AccountRecord | LiquidityPoolRecord>;
            counter: CallFunction<AccountRecord | LiquidityPoolRecord>;
        }
        export {};
    }
    export type TradeRecord = TradeRecord.Orderbook | TradeRecord.LiquidityPool;
    export interface TransactionRecord extends Omit<HorizonApi.TransactionResponse, "ledger"> {
        ledger_attr: HorizonApi.TransactionResponse["ledger"];
        account: CallFunction<AccountRecord>;
        effects: CallCollectionFunction<EffectRecord>;
        ledger: CallFunction<LedgerRecord>;
        operations: CallCollectionFunction<OperationRecord>;
        precedes: CallFunction<TransactionRecord>;
        self: CallFunction<TransactionRecord>;
        succeeds: CallFunction<TransactionRecord>;
    }
    export interface OrderbookRecord extends HorizonApi.BaseResponse {
        bids: Array<{
            price_r: {
                d: number;
                n: number;
            };
            price: string;
            amount: string;
        }>;
        asks: Array<{
            price_r: {
                d: number;
                n: number;
            };
            price: string;
            amount: string;
        }>;
        base: Asset;
        counter: Asset;
    }
    export interface PaymentPathRecord extends HorizonApi.BaseResponse {
        path: Array<{
            asset_code: string;
            asset_issuer: string;
            asset_type: string;
        }>;
        source_amount: string;
        source_asset_type: string;
        source_asset_code: string;
        source_asset_issuer: string;
        destination_amount: string;
        destination_asset_type: string;
        destination_asset_code: string;
        destination_asset_issuer: string;
    }
    export {};
}
