use astroport::asset::{Asset, AssetInfo};
use injective_cosmwasm::MarketId;
use injective_math::FPDecimal;
use pyth_sdk_cw::PriceIdentifier;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::Item;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Config {
    pub lp_token: Addr,
    pub deposit_asset: AssetInfo,
    pub pyth_contract_addr: Addr,
}

pub const CONFIG: Item<Config> = Item::new("config");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BasketAsset {
    pub asset: Asset,
    pub weight: Uint128,
    pub pyth_price_feed: PriceIdentifier,
    pub spot_market_id: MarketId,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Basket {
    pub assets: Vec<BasketAsset>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ClobCache {
    pub quantity: FPDecimal,
    pub price: FPDecimal,
    pub fee: FPDecimal,
}

impl ClobCache {
    pub fn new() -> Self {
        ClobCache {
            quantity: FPDecimal::zero(),
            price: FPDecimal::zero(),
            fee: FPDecimal::zero(),
        }
    }
}

pub const BASKET: Item<Basket> = Item::new("basket");
pub const CLOB_CACHE: Item<Vec<ClobCache>> = Item::new("clob-cache");
