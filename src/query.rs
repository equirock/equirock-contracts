use cosmwasm_std::{Addr, Decimal, Deps, Env, QuerierWrapper, StdError, StdResult, Uint128};
use pyth_sdk_cw::{query_price_feed, Price, PriceFeedResponse, PriceIdentifier};

use crate::{
    msg::{FetchPriceResponse, GetBasketAssetIdealRatioResponse},
    state::{Basket, BasketAsset, Config, BASKET, CONFIG},
};

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
    env: &Env,
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

    let basket_asset_value = price.checked_mul(Decimal::raw(basket_asset.asset.amount.into()))?;

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