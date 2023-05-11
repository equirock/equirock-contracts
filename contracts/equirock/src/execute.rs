use std::{
    ops::{Div, Sub},
    str::FromStr,
};

use astroport::asset::{Asset, AssetInfo};
use cosmwasm_std::{
    to_binary, Addr, CosmosMsg, Decimal, DepsMut, Env, MessageInfo, Response, StdError, StdResult,
    SubMsg, WasmMsg,
};
use injective_cosmwasm::{
    create_spot_market_order_msg, get_default_subaccount_id_for_checked_address,
    InjectiveMsgWrapper, InjectiveQuerier, InjectiveQueryWrapper, MarketId, OrderType, SpotMarket,
    SpotOrder, SubaccountId,
};
use injective_math::FPDecimal;

use crate::{
    msg::{CallbackMsg, ExecuteMsg},
    querier::{query_balance, query_decimals},
    query::{basket_value_usdt, get_basket_ideal_ratio},
    reply::ATOMIC_ORDER_REPLY_ID,
    state::{ClobCache, BASKET, CLOB_CACHE, CONFIG},
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
    order_type: OrderType,
    subaccount_id: &SubaccountId,
    sender: &Addr,
) -> StdResult<CosmosMsg<InjectiveMsgWrapper>> {
    let price_s = price.checked_mul(slippage)?;

    let price_scale_factor = FPDecimal::from(10_i128.pow((base_decimals - quote_decimals) as u32));
    let mut price_fp = FPDecimal::from_str(&price_s.to_string())?.div(price_scale_factor);
    price_fp = market.min_price_tick_size * (price_fp.div(market.min_price_tick_size)).int();

    let quantity_s = if slippage.gt(&Decimal::one()) {
        quantity
            .checked_div(slippage)
            .map_err(|e| StdError::GenericErr { msg: e.to_string() })?
    } else {
        quantity
    };

    let quantity_scale_factor = FPDecimal::from(10_i128.pow(base_decimals as u32));
    let mut quantity_fp = FPDecimal::from_str(&quantity_s.to_string())? * quantity_scale_factor;
    quantity_fp =
        market.min_quantity_tick_size * (quantity_fp.div(market.min_quantity_tick_size)).int();

    let order = SpotOrder::new(
        price_fp,
        quantity_fp,
        order_type,
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
            return Err(StdError::generic_err(format!(
                "Deposit other tokens {}",
                other_coin
            )));
        }
    }

    asset.assert_sent_native_token_balance(&info)?;

    CLOB_CACHE.save(deps.storage, &vec![ClobCache::new()])?;

    let basket = BASKET.load(deps.storage)?;

    let contract = &env.contract.address;

    let asset_ideals = get_basket_ideal_ratio(deps.as_ref(), &env)?;

    let subaccount_id = get_default_subaccount_id_for_checked_address(contract);
    let slippage = Decimal::from_ratio(5u128, 100u128).checked_add(Decimal::one())?;
    let mut submessages: Vec<SubMsg<InjectiveMsgWrapper>> = vec![];
    let injective_querier = InjectiveQuerier::new(&deps.querier);

    let mut log: Vec<String> = vec![];
    for asset_ideal in asset_ideals {
        let market =
            injective_querier.query_spot_market(&asset_ideal.basket_asset.spot_market_id)?;
        if let Some(market) = market.market {
            log.push(format!("{:?}", market));
            let base_decimals = query_decimals(
                &deps.querier,
                &AssetInfo::NativeToken {
                    denom: market.base_denom.to_owned(),
                },
            );
            let quote_decimals = 6u64; // USDT

            log.push(format!("base_decimals {:?}", base_decimals));
            log.push(format!("asset_ideal {:?}", asset_ideal));

            let order_msg = spot_order(
                slippage,
                asset_ideal.price,
                asset_ideal.ratio.checked_mul(
                    Decimal::from_atomics(asset.amount, quote_decimals as u32)
                        .map_err(|e| StdError::generic_err(e.to_string()))?,
                    // .checked_mul(Decimal::from_str("0.998")?)?,
                )?,
                &market,
                base_decimals,
                quote_decimals,
                OrderType::BuyAtomic,
                &subaccount_id,
                contract,
            )?;

            log.push(format!("order_msg {:?}", order_msg));

            let order_message = SubMsg::reply_on_success(order_msg, ATOMIC_ORDER_REPLY_ID);
            submessages.push(order_message);
        }
    }

    let basket_value_in_usdt = basket_value_usdt(&deps.querier, &env, &config, &basket)?;

    let mut messages: Vec<CosmosMsg<InjectiveMsgWrapper>> = vec![];
    let after_deposit_msg = CosmosMsg::<InjectiveMsgWrapper>::Wasm(WasmMsg::Execute {
        contract_addr: contract.to_owned().into_string(),
        msg: to_binary(&ExecuteMsg::Callback(CallbackMsg::AfterDeposit {
            deposit: asset.amount,
            sender: info.sender.to_owned(),
            basket_value: basket_value_in_usdt,
        }))?,
        funds: vec![],
    });

    if info.sender != env.contract.address {
        messages.push(after_deposit_msg);
    }

    Ok(Response::new()
        .add_attribute("action", "deposit")
        .add_attribute("log", format!("{:?}", log))
        .add_submessages(submessages)
        .add_messages(messages))
}

pub fn rebalance(
    deps: DepsMut<InjectiveQueryWrapper>,
    env: Env,
    _info: MessageInfo,
) -> Result<Response<InjectiveMsgWrapper>, StdError> {
    let config = CONFIG.load(deps.storage)?;

    CLOB_CACHE.save(deps.storage, &vec![ClobCache::new()])?;

    let basket = BASKET.load(deps.storage)?;

    let contract = &env.contract.address;

    let asset_ideals = get_basket_ideal_ratio(deps.as_ref(), &env)?;

    let subaccount_id = get_default_subaccount_id_for_checked_address(contract);
    let slippage = Decimal::from_ratio(15u128, 100u128).checked_add(Decimal::one())?;
    let mut submessages: Vec<SubMsg<InjectiveMsgWrapper>> = vec![];
    let injective_querier = InjectiveQuerier::new(&deps.querier);

    let basket_value = basket_value_usdt(&deps.querier, &env, &config, &basket)?;
    let usdt_decimals = 6u64;

    let mut log: Vec<String> = vec![];

    for asset_ideal in asset_ideals {
        let market =
            injective_querier.query_spot_market(&asset_ideal.basket_asset.spot_market_id)?;

        if let Some(market) = market.market {
            let amount = query_balance(
                &deps.querier,
                &asset_ideal.basket_asset.asset.info,
                &env.contract.address,
            )?;
            let decimals = query_decimals(&deps.querier, &asset_ideal.basket_asset.asset.info);
            let current_quantity = Decimal::from_atomics(amount, decimals as u32).unwrap();

            let ideal_quantity = asset_ideal
                .ratio
                .checked_mul(Decimal::from_atomics(basket_value, usdt_decimals as u32).unwrap())?;

            // less then ==> Sell
            if ideal_quantity.lt(&current_quantity) {
                let diff = current_quantity.sub(ideal_quantity);
                let order_msg = spot_order(
                    slippage,
                    asset_ideal.price,
                    diff,
                    &market,
                    decimals,
                    usdt_decimals,
                    OrderType::SellAtomic,
                    &subaccount_id,
                    contract,
                )?;

                log.push(format!("order_msg {:?}", order_msg));

                let order_message = SubMsg::reply_on_success(order_msg, ATOMIC_ORDER_REPLY_ID);
                submessages.push(order_message);
            }
        }
    }

    let after_rebalance_msg = WasmMsg::Execute {
        contract_addr: contract.to_owned().into_string(),
        msg: to_binary(&ExecuteMsg::Callback(CallbackMsg::AfterRebalanceSell {}))?,
        funds: vec![],
    };

    Ok(Response::new()
        .add_attribute("action", "rebalance")
        .add_attribute("log", format!("{:?}", log))
        .add_submessages(submessages)
        .add_message(after_rebalance_msg))
}
