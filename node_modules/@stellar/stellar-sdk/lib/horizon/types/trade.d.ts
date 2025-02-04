import { BaseEffectRecord, EffectType } from "./effects";
export interface Trade extends BaseEffectRecord<'trade'> {
    type_i: EffectType.trade;
    seller: string;
    offer_id: number | string;
    bought_amount: string;
    bought_asset_type: string;
    bought_asset_code: string;
    bought_asset_issuer: string;
    sold_amount: string;
    sold_asset_type: string;
    sold_asset_code: string;
    sold_asset_issuer: string;
}
