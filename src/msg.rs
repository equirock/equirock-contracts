use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal};
use pyth_sdk_cw::Price;

use crate::state::{Asset, AssetInfo, Basket, BasketAsset, Config};

#[cw_serde]
pub struct InstantiateMsg {
    pub etf_token_code_id: u64,
    pub etf_token_name: String,
    pub deposit_asset: AssetInfo,
    pub pyth_contract_addr: Addr,
    pub basket: Basket,
}

#[cw_serde]
pub enum ExecuteMsg {
    UpdateConfig {},
    Deposit { asset: Asset },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    // GetCount returns the current count as a json-encoded number
    #[returns(Config)]
    GetConfig {},
    #[returns(Vec<GetBasketAssetIdealRatioResponse>)]
    GetBasketIdealRatio {},
}

#[cw_serde]
pub struct GetBasketAssetIdealRatioResponse {
    pub basket_asset: BasketAsset,
    pub ratio: Decimal,
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub struct FetchPriceResponse {
    pub current_price: Price,
    pub ema_price: Price,
}
