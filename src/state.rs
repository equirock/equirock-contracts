use std::fmt;

use cosmwasm_schema::cw_serde;
use pyth_sdk_cw::PriceIdentifier;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, CanonicalAddr, MessageInfo, StdError, StdResult, Uint128};
use cw_storage_plus::Item;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Config {
    pub lp_token: CanonicalAddr,
    pub deposit_asset: AssetInfo,
    pub pyth_contract_addr: Addr,
}

pub const CONFIG: Item<Config> = Item::new("config");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct BasketAsset {
    pub asset: Asset,
    pub weight: Uint128,
    pub pyth_price_feed: PriceIdentifier,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Basket {
    pub assets: Vec<BasketAsset>,
}

pub const BASKET: Item<Basket> = Item::new("basket");

#[cw_serde]
#[derive(Eq)]
pub struct Asset {
    pub info: AssetInfo,
    pub amount: Uint128,
}

impl Asset {
    pub fn assert_sent_native_token_balance(&self, message_info: &MessageInfo) -> StdResult<()> {
        if let AssetInfo::NativeToken { denom } = &self.info {
            match message_info.funds.iter().find(|x| x.denom == *denom) {
                Some(coin) => {
                    if self.amount == coin.amount {
                        Ok(())
                    } else {
                        Err(StdError::generic_err("Native token balance mismatch between the argument and the transferred"))
                    }
                }
                None => {
                    if self.amount.is_zero() {
                        Ok(())
                    } else {
                        Err(StdError::generic_err("Native token balance mismatch between the argument and the transferred"))
                    }
                }
            }
        } else {
            Ok(())
        }
    }
}

#[cw_serde]
#[derive(Eq)]
pub enum AssetInfo {
    Token { contract_addr: String },
    NativeToken { denom: String },
}

impl fmt::Display for AssetInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AssetInfo::NativeToken { denom } => write!(f, "{}", denom),
            AssetInfo::Token { contract_addr } => write!(f, "{}", contract_addr),
        }
    }
}
