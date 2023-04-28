use std::{ops::Div, str::FromStr};

use astroport::asset::{Asset, AssetInfo};
use cosmwasm_std::{
    Addr, CosmosMsg, Decimal, DepsMut, Env, MessageInfo, Response, StdError, StdResult, SubMsg,
};
use injective_cosmwasm::{
    create_spot_market_order_msg, get_default_subaccount_id_for_checked_address,
    InjectiveMsgWrapper, InjectiveQuerier, InjectiveQueryWrapper, MarketId, OrderType, SpotMarket,
    SpotOrder, SubaccountId,
};
use injective_math::FPDecimal;

use crate::{
    querier::query_token_info,
    query::{basket_value, get_basket_ideal_ratio},
    reply::ATOMIC_ORDER_REPLY_ID,
    state::{BASKET, CONFIG, REPLY_CACHE},
};

pub fn update_config(
    deps: DepsMut<InjectiveQueryWrapper>,
    _info: MessageInfo,
    _new_owner: Option<Addr>,
    _new_dens_addr: Option<Addr>,
) -> Result<Response<InjectiveMsgWrapper>, StdError> {
    let _config = CONFIG.update(deps.storage, |config| -> Result<_, StdError> { Ok(config) })?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

pub fn sell_inj_spot_order(
    market_id: &MarketId,
    quantity: FPDecimal,
    subaccount_id: &SubaccountId,
    sender: &Addr,
) -> CosmosMsg<InjectiveMsgWrapper> {
    let order = SpotOrder::new(
        FPDecimal::zero(),
        quantity,
        OrderType::SellAtomic,
        market_id,
        subaccount_id.to_owned(),
        Some(sender.to_owned()),
    );

    create_spot_market_order_msg(sender.to_owned(), order)
}

pub fn buy_inj_spot_order(
    market_id: &MarketId,
    price: FPDecimal,
    quantity: FPDecimal,
    subaccount_id: &SubaccountId,
    sender: &Addr,
) -> CosmosMsg<InjectiveMsgWrapper> {
    let order = SpotOrder::new(
        price,
        quantity,
        OrderType::BuyAtomic,
        market_id,
        subaccount_id.to_owned(),
        Some(sender.to_owned()),
    );

    create_spot_market_order_msg(sender.to_owned(), order)
}

pub fn spot_order(
    slippage: Decimal,
    price: Decimal,
    quantity: Decimal,
    market: &SpotMarket,
    base_decimals: u64,
    quote_decimals: u64,
    subaccount_id: &SubaccountId,
    sender: &Addr,
) -> StdResult<CosmosMsg<InjectiveMsgWrapper>> {
    let slippage_perc = slippage.checked_add(Decimal::one())?;
    let price_s = price.checked_mul(slippage_perc)?;

    let price_scale_factor = FPDecimal::from(10_i128.pow((base_decimals - quote_decimals) as u32));
    let mut price_fp = FPDecimal::from_str(&price_s.to_string())?.div(price_scale_factor);
    price_fp = market.min_price_tick_size * (price_fp.div(market.min_price_tick_size)).int();

    let quantity_s = quantity
        .checked_div(slippage_perc)
        .map_err(|e| StdError::GenericErr { msg: e.to_string() })?;

    let quantity_scale_factor = FPDecimal::from(10_i128.pow(base_decimals as u32));
    let mut quantity_fp = FPDecimal::from_str(&quantity_s.to_string())? * quantity_scale_factor;
    quantity_fp =
        market.min_quantity_tick_size * (quantity_fp.div(market.min_quantity_tick_size)).int();

    let order = SpotOrder::new(
        price_fp,
        quantity_fp,
        OrderType::BuyAtomic,
        &market.market_id,
        subaccount_id.to_owned(),
        Some(sender.to_owned()),
    );

    Ok(create_spot_market_order_msg(sender.to_owned(), order))
}

pub fn deposit(
    deps: DepsMut<InjectiveQueryWrapper>,
    env: Env,
    info: MessageInfo,
    asset: Asset,
) -> Result<Response<InjectiveMsgWrapper>, StdError> {
    let config = CONFIG.load(deps.storage)?;

    if let AssetInfo::NativeToken { denom } = &config.deposit_asset {
        if let Some(other_coin) = info.funds.iter().find(|x| x.denom != *denom) {
            return Err(
                StdError::generic_err(format!("Deposit other tokens {}", other_coin)).into(),
            );
        }
    }

    asset.assert_sent_native_token_balance(&info)?;

    REPLY_CACHE.save(deps.storage, &asset)?;

    let basket = BASKET.load(deps.storage)?;

    let _basket_value = basket_value(&deps.querier, &env, &config, &basket)?;

    let liquidity_token = deps.api.addr_humanize(&config.lp_token)?;
    let _total_share = query_token_info(&deps.querier, liquidity_token)?.total_supply;

    let contract = &env.contract.address;

    let asset_ideals = get_basket_ideal_ratio(deps.as_ref(), &env)?;

    let _deposit_without_fee =
        Decimal::raw(asset.amount.into()).checked_mul(Decimal::from_str("0.998")?)?;

    let subaccount_id = get_default_subaccount_id_for_checked_address(&contract);
    let slippage = Decimal::from_ratio(1u128, 100u128);
    let mut submessages: Vec<SubMsg<InjectiveMsgWrapper>> = vec![];
    let injective_querier = InjectiveQuerier::new(&deps.querier);

    for asset_ideal in asset_ideals {
        let market =
            injective_querier.query_spot_market(&asset_ideal.basket_asset.spot_market_id)?;
        if let Some(market) = market.market {
            let denom_decimals = injective_querier
                .query_denom_decimals::<Vec<String>>(&vec![
                    market.base_denom.to_owned(),
                    market.quote_denom.to_owned(),
                ])?
                .denom_decimals;

            let base_decimals = denom_decimals[0].decimals;
            let quote_decimals = denom_decimals[1].decimals;

            let order_message = SubMsg::reply_on_success(
                spot_order(
                    slippage,
                    asset_ideal.price,
                    asset_ideal.ratio.checked_mul(
                        Decimal::from_atomics(asset.amount, quote_decimals as u32)
                            .map_err(|e| StdError::generic_err(e.to_string()))?,
                    )?,
                    &market,
                    base_decimals,
                    quote_decimals,
                    &subaccount_id,
                    &contract,
                )?,
                ATOMIC_ORDER_REPLY_ID,
            );
            submessages.push(order_message);
        }
    }

    Ok(Response::new()
        .add_attribute("action", "deposit")
        .add_submessages(submessages))

    // let spot_market = injective_querier.query_spot_market(&basket.assets[0].spot_market_id)?;

    // if let Some(market) = spot_market.market {
    //     market.min_quantity_tick_size;
    // }

    // let order_message = SubMsg::reply_on_success(
    //     buy_inj_spot_order(
    //         &MarketId::new("0x0611780ba69656949525013d947713300f56c37b6175e02f26bffa495c3208fe")?,
    //         FPDecimal::from_str("0.000000000007743000")?,
    //         FPDecimal::from_str("14000000000000000.000000000000000000")?,
    //         &subaccount_id,
    //         contract,
    //     ),
    //     ATOMIC_ORDER_REPLY_ID,
    // );

    // Ok(Response::new()
    //     .add_attribute("action", "deposit")
    //     .add_submessage(order_message))
}
