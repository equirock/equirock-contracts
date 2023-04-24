use astroport::asset::AssetInfo;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, CanonicalAddr, Decimal, Deps, DepsMut, Env, MessageInfo, Reply,
    ReplyOn, Response, StdError, StdResult, SubMsg, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw20_base::msg::InstantiateMsg as CW20InstantiateMsg;
use protobuf::Message;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::{Basket, Config, BASKET, CONFIG};

use self::execute::{deposit, update_config};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:equirock-contract";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const INSTANTIATE_REPLY_ID: u64 = 1;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    if let AssetInfo::Token { .. } = &msg.deposit_asset {
        return Err(StdError::generic_err("Tokens not supported").into());
    }

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        lp_token: CanonicalAddr::from(vec![]),
        deposit_asset: msg.deposit_asset,
        pyth_contract_addr: msg.pyth_contract_addr,
    };

    CONFIG.save(deps.storage, &config)?;

    if let Some(non_zero_basket_asset) = msg
        .basket
        .assets
        .iter()
        .find(|b| b.asset.amount != Uint128::zero())
    {
        return Err(StdError::generic_err(format!(
            "Non-zero basket asset {}",
            non_zero_basket_asset.asset.info
        ))
        .into());
    }

    BASKET.save(deps.storage, &msg.basket)?;

    Ok(Response::new().add_submessage(SubMsg {
        // Create LP token
        msg: WasmMsg::Instantiate {
            admin: None,
            code_id: msg.etf_token_code_id,
            msg: to_binary(&CW20InstantiateMsg {
                name: msg.etf_token_name.clone(),
                symbol: "uER".to_string(),
                decimals: 6,
                initial_balances: vec![],
                mint: Some(cw20::MinterResponse {
                    minter: env.contract.address.to_string(),
                    cap: None,
                }),
                marketing: None,
            })?,
            funds: vec![],
            label: msg.etf_token_name,
        }
        .into(),
        gas_limit: None,
        id: INSTANTIATE_REPLY_ID,
        reply_on: ReplyOn::Success,
    }))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig {} => update_config(deps, info, None, None),
        ExecuteMsg::Deposit { asset } => deposit(deps, env, info, asset),
    }
}

pub mod execute {
    use astroport::{asset::Asset, pair::ExecuteMsg::Swap};
    use cosmwasm_std::{Addr, QueryRequest, WasmQuery};
    use cw721::{Cw721QueryMsg, OwnerOfResponse};

    use crate::{
        querier::query_token_info,
        state::{BASKET, CONFIG},
    };

    use super::{query::basket_value, *};

    pub fn update_config(
        deps: DepsMut,
        info: MessageInfo,
        new_owner: Option<Addr>,
        new_dens_addr: Option<Addr>,
    ) -> Result<Response, ContractError> {
        let config = CONFIG.update(deps.storage, |mut config| -> Result<_, ContractError> {
            Ok(config)
        })?;

        Ok(Response::new().add_attribute("action", "update_config"))
    }

    pub fn create_astroport_swap_msg() -> StdResult<()> {
        // Swap {
        //     offer_asset: (),
        //     ask_asset_info: (),
        //     belief_price: (),
        //     max_spread: (),
        //     to: (),
        // };

        Ok(())
    }

    pub fn deposit(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        asset: Asset,
    ) -> Result<Response, ContractError> {
        let config = CONFIG.load(deps.storage)?;

        if let AssetInfo::NativeToken { denom } = &config.deposit_asset {
            if let Some(other_coin) = info.funds.iter().find(|x| x.denom != *denom) {
                return Err(
                    StdError::generic_err(format!("Deposit other tokens {}", other_coin)).into(),
                );
            }
        }

        asset.assert_sent_native_token_balance(&info)?;

        let basket = BASKET.load(deps.storage)?;

        let basket_value = basket_value(&deps.querier, env, &config, &basket)?;

        let liquidity_token = deps.api.addr_humanize(&config.lp_token)?;
        let total_share = query_token_info(&deps.querier, liquidity_token)?.total_supply;

        Ok(Response::new().add_attribute("action", "deposit"))
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig {} => to_binary(&query::config(deps)?),
        QueryMsg::GetBasketIdealRatio {} => to_binary(&query::get_basket_ideal_ratio(deps, env)?),
    }
}

pub mod query {

    use cosmwasm_std::{Querier, QuerierWrapper, Uint128};
    use pyth_sdk_cw::{query_price_feed, Price, PriceFeedResponse, PriceIdentifier};

    use crate::{
        msg::{FetchPriceResponse, GetBasketAssetIdealRatioResponse},
        state::{Basket, BasketAsset, Config, CONFIG},
    };

    use super::*;

    pub fn config(deps: Deps) -> StdResult<Config> {
        let config = CONFIG.load(deps.storage)?;
        Ok(config)
    }

    pub fn get_basket_ideal_ratio(
        deps: Deps,
        env: Env,
    ) -> StdResult<Vec<GetBasketAssetIdealRatioResponse>> {
        let config = CONFIG.load(deps.storage)?;
        let basket = BASKET.load(deps.storage)?;

        let ratios = basket_ideal_state(&deps.querier, env, &config, &basket)?;

        Ok(basket
            .assets
            .iter()
            .zip(ratios)
            .map(|(basket_asset, ratio)| GetBasketAssetIdealRatioResponse {
                basket_asset: basket_asset.to_owned(),
                ratio,
            })
            .collect())
    }

    pub fn basket_ideal_state(
        querier: &QuerierWrapper,
        env: Env,
        config: &Config,
        basket: &Basket,
    ) -> StdResult<Vec<Decimal>> {
        let w_sum = basket
            .assets
            .iter()
            .try_fold(Uint128::zero(), |acc, basket_asset| {
                acc.checked_add(basket_asset.weight)
            })?;

        let basket_asset_ratios = basket
            .assets
            .iter()
            .map(|basket_asset| basket_asset_ratio(querier, &env, &config, basket_asset, w_sum))
            .collect::<StdResult<Vec<Decimal>>>()?;

        Ok(basket_asset_ratios)
    }

    pub fn basket_asset_ratio(
        querier: &QuerierWrapper,
        env: &Env,
        config: &Config,
        basket_asset: &BasketAsset,
        w_sum: Uint128,
    ) -> StdResult<Decimal> {
        let fetch_price = basket_asset_price(
            querier,
            env,
            &config.pyth_contract_addr,
            basket_asset.pyth_price_feed,
        )?;

        let price = pyth_price(fetch_price.current_price)?;

        let basket_asset_ratio = Decimal::from_ratio(
            basket_asset.weight,
            w_sum, // w_sum.checked_mul(Uint128::from(fetch_price.current_price.price as u128))?,
        )
        .checked_div(price)
        .map_err(|e| StdError::generic_err(e.to_string()))?;

        Ok(basket_asset_ratio)
    }

    pub fn basket_value(
        querier: &QuerierWrapper,
        env: Env,
        config: &Config,
        basket: &Basket,
    ) -> StdResult<Decimal> {
        let basket_asset_values = basket
            .assets
            .iter()
            .map(|basket_asset| basket_asset_value(querier, &env, &config, basket_asset))
            .collect::<StdResult<Vec<Decimal>>>()?;

        let sum = basket_asset_values
            .iter()
            .try_fold(Decimal::zero(), |acc, value| acc.checked_add(*value))?;

        Ok(sum)
    }

    pub fn pyth_price(price: Price) -> StdResult<Decimal> {
        let price_price = Uint128::from(price.price as u128);
        let price_expo = price.expo;

        if price_expo < 0 {
            Ok(Decimal::from_ratio(
                price_price,
                Uint128::from(10u128).checked_mul(Uint128::from(price_expo.abs() as u128))?,
            ))
        } else if price_expo == 0 {
            Ok(Decimal::raw(price_price.into()))
        } else {
            Ok(Decimal::raw(
                (price_price.checked_mul(
                    Uint128::from(10u128).checked_mul(Uint128::from(price_expo as u128))?,
                )?)
                .into(),
            ))
        }
    }

    pub fn basket_asset_value(
        querier: &QuerierWrapper,
        env: &Env,
        config: &Config,
        basket_asset: &BasketAsset,
    ) -> StdResult<Decimal> {
        let fetch_price = basket_asset_price(
            querier,
            env,
            &config.pyth_contract_addr,
            basket_asset.pyth_price_feed,
        )?;
        let price = pyth_price(fetch_price.current_price)?;

        let basket_asset_value =
            price.checked_mul(Decimal::raw(basket_asset.asset.amount.into()))?;

        Ok(basket_asset_value)
    }

    pub fn basket_asset_price(
        querier: &QuerierWrapper,
        env: &Env,
        pyth_contract_addr: &Addr,
        price_feed_id: PriceIdentifier,
    ) -> StdResult<FetchPriceResponse> {
        let price_feed_response: PriceFeedResponse =
            query_price_feed(querier, pyth_contract_addr.to_owned(), price_feed_id)?;
        let price_feed = price_feed_response.price_feed;

        let current_price = price_feed
            .get_price_no_older_than(env.block.time.seconds() as i64, 60)
            .ok_or_else(|| StdError::not_found("Current price is not available"))?;

        let ema_price = price_feed
            .get_ema_price_no_older_than(env.block.time.seconds() as i64, 60)
            .ok_or_else(|| StdError::not_found("EMA price is not available"))?;

        Ok(FetchPriceResponse {
            current_price,
            ema_price,
        })
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> StdResult<Response> {
    use crate::response::MsgInstantiateContractResponse;

    let data = msg.result.unwrap().data.unwrap();
    let res: MsgInstantiateContractResponse =
        Message::parse_from_bytes(data.as_slice()).map_err(|_| {
            StdError::parse_err("MsgInstantiateContractResponse", "failed to parse data")
        })?;
    let liquidity_token = res.address;

    let api = deps.api;
    CONFIG.update(deps.storage, |mut config| -> StdResult<_> {
        config.lp_token = api.addr_canonicalize(&liquidity_token)?;
        Ok(config)
    })?;

    Ok(Response::new().add_attribute("liquidity_token_addr", liquidity_token))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}
