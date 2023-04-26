use std::str::FromStr;

use astroport::asset::{Asset, AssetInfo};
use cosmwasm_std::{
    Addr, CosmosMsg, Decimal, DepsMut, Env, MessageInfo, Response, StdError, SubMsg,
};
use injective_cosmwasm::{
    create_spot_market_order_msg, get_default_subaccount_id_for_checked_address,
    InjectiveMsgWrapper, InjectiveQuerier, InjectiveQueryWrapper, MarketId, OrderType, SpotOrder,
    SubaccountId,
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

    let ratios = get_basket_ideal_ratio(deps.as_ref(), &env)?;

    let _response: Response<InjectiveMsgWrapper> = Response::new();

    let _deposit_without_fee =
        Decimal::raw(asset.amount.into()).checked_mul(Decimal::from_str("0.998")?)?;

    for _ratio in ratios {}

    let _injective_querier = InjectiveQuerier::new(&deps.querier);

    // let spot_market = injective_querier.query_spot_market(&basket.assets[0].spot_market_id)?;

    // if let Some(market) = spot_market.market {
    //     market.min_quantity_tick_size;
    // }

    let subaccount_id = get_default_subaccount_id_for_checked_address(&contract);
    let order_message = SubMsg::reply_on_success(
        buy_inj_spot_order(
            &MarketId::new("0x0611780ba69656949525013d947713300f56c37b6175e02f26bffa495c3208fe")?,
            FPDecimal::from_str("0.000000000007743000")?,
            FPDecimal::from_str("14000000000000000.000000000000000000")?,
            &subaccount_id,
            contract,
        ),
        ATOMIC_ORDER_REPLY_ID,
    );

    Ok(Response::new()
        .add_attribute("action", "deposit")
        .add_submessage(order_message))
}
