import { AssetType } from "@stellar/stellar-base";
import { HorizonApi } from "../horizon_api";
export interface AssetRecord extends HorizonApi.BaseResponse {
    asset_type: AssetType.credit4 | AssetType.credit12;
    asset_code: string;
    asset_issuer: string;
    paging_token: string;
    accounts: HorizonApi.AssetAccounts;
    balances: HorizonApi.AssetBalances;
    num_claimable_balances: number;
    num_liquidity_pools: number;
    num_contracts: number;
    claimable_balances_amount: string;
    liquidity_pools_amount: string;
    contracts_amount: string;
    flags: HorizonApi.Flags;
}
