use astroport::asset::{Asset, AssetInfo};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Uint128};
use cw20::Cw20ReceiveMsg;
use pyth_sdk_cw::Price;

use crate::state::{Basket, BasketAsset, Config};

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
    Rebalance {},
    Callback(CallbackMsg),
    Receive(Cw20ReceiveMsg),
}

#[cw_serde]
pub enum Cw20HookMsg {
    /// Withdraws a given amount from the vault.
    Withdraw {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    // GetCount returns the current count as a json-encoded number
    #[returns(Config)]
    GetConfig {},
    #[returns(Vec<GetBasketAssetIdealRatioResponse>)]
    GetBasketIdealRatio {},
    #[returns(Uint128)]
    GetBasketValueInUsdt {},
}

#[cw_serde]
pub struct GetBasketAssetIdealRatioResponse {
    pub basket_asset: BasketAsset,
    pub ratio: Decimal,
    pub price: Decimal,
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub struct FetchPriceResponse {
    pub current_price: Price,
    pub ema_price: Price,
}

#[cw_serde]
pub enum CallbackMsg {
    AfterDeposit {
        deposit: Uint128,
        sender: Addr,
        basket_value: Uint128,
    },
    AfterWithdraw {
        sender: Addr,
    },
    AfterRebalanceSell {},
}
