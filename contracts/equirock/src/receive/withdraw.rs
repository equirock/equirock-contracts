use astroport::asset::AssetInfo;
use cosmwasm_std::{
    to_binary, CosmosMsg, Decimal, DepsMut, Env, Response, StdError, StdResult, SubMsg, Uint128,
    WasmMsg,
};
use cw20::{Cw20ExecuteMsg, Cw20QueryMsg, TokenInfoResponse};
use injective_cosmwasm::{
    get_default_subaccount_id_for_checked_address, InjectiveMsgWrapper, InjectiveQuerier,
    InjectiveQueryWrapper, OrderType,
};

use crate::{
    execute::spot_order,
    msg::{CallbackMsg, ExecuteMsg},
    querier::{query_balance, query_decimals},
    query::{basket_asset_price, pyth_price},
    reply::ATOMIC_ORDER_REPLY_ID,
    state::{ClobCache, BASKET, CLOB_CACHE, CONFIG},
};

pub fn withdraw(
    deps: DepsMut<InjectiveQueryWrapper>,
    env: Env,
    sender: String,
    amount: Uint128,
) -> StdResult<Response<InjectiveMsgWrapper>> {
    let config = CONFIG.load(deps.storage)?;

    let sender = deps.api.addr_validate(&sender)?;
    // let collected_protocol_fees = COLLECTED_PROTOCOL_FEES.load(deps.storage)?;
    // let total_asset_amount = match &config.asset_info {
    //     AssetInfo::NativeToken { denom } => {
    //         deps.querier
    //             .query_balance(env.contract.address, denom)?
    //             .amount
    //     }
    //     AssetInfo::Token { contract_addr } => {
    //         let balance: BalanceResponse = deps.querier.query_wasm_smart(
    //             contract_addr,
    //             &Cw20QueryMsg::Balance {
    //                 address: env.contract.address.into_string(),
    //             },
    //         )?;
    //         balance.balance
    //     }
    // } // deduct protocol fees
    // .checked_sub(collected_protocol_fees.amount)?;

    let total_share: TokenInfoResponse = deps
        .querier
        .query_wasm_smart(&config.lp_token, &Cw20QueryMsg::TokenInfo {})?;

    CLOB_CACHE.save(deps.storage, &vec![ClobCache::new()])?;

    let basket = BASKET.load(deps.storage)?;
    let withdraw_ratio = Decimal::from_ratio(amount, total_share.total_supply);

    let contract = &env.contract.address;
    let subaccount_id = get_default_subaccount_id_for_checked_address(contract);
    let slippage = Decimal::one().checked_sub(Decimal::from_ratio(5u128, 100u128))?;
    let mut submessages: Vec<SubMsg<InjectiveMsgWrapper>> = vec![];
    let injective_querier = InjectiveQuerier::new(&deps.querier);

    let mut log: Vec<String> = vec![];
    for basket_asset in basket.assets {
        let market = injective_querier.query_spot_market(&basket_asset.spot_market_id)?;
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

            let fetch_price = basket_asset_price(
                &deps.querier,
                &env,
                &config.pyth_contract_addr,
                basket_asset.pyth_price_feed,
            )?;
            let price = pyth_price(fetch_price.current_price)?;

            let amount = query_balance(
                &deps.querier,
                &basket_asset.asset.info,
                &env.contract.address,
            )?;

            let order_msg = spot_order(
                slippage,
                price,
                withdraw_ratio.checked_mul(
                    Decimal::from_atomics(amount, base_decimals as u32)
                        .map_err(|e| StdError::generic_err(e.to_string()))?,
                    // .checked_mul(Decimal::from_str("0.998")?)?,
                )?,
                &market,
                base_decimals,
                quote_decimals,
                OrderType::SellAtomic,
                &subaccount_id,
                contract,
            )?;

            log.push(format!("order_msg {:?}", order_msg));

            let order_message = SubMsg::reply_on_success(order_msg, ATOMIC_ORDER_REPLY_ID);
            submessages.push(order_message);
        }
    }

    let after_withdraw_msg = WasmMsg::Execute {
        contract_addr: contract.to_owned().into_string(),
        msg: to_binary(&ExecuteMsg::Callback(CallbackMsg::AfterWithdraw { sender }))?,
        funds: vec![],
    };

    let burn_lp_tokens_msg = WasmMsg::Execute {
        contract_addr: config.lp_token.into_string(),
        funds: vec![],
        msg: to_binary(&Cw20ExecuteMsg::Burn { amount })?,
    };

    let messages: Vec<CosmosMsg<InjectiveMsgWrapper>> =
        vec![after_withdraw_msg.into(), burn_lp_tokens_msg.into()];

    Ok(Response::new()
        .add_attributes(vec![("method", "withdraw")])
        .add_submessages(submessages)
        .add_messages(messages))
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use astroport::asset::{Asset, AssetInfo};
    use cosmwasm_std::{coins, testing::mock_info, to_binary, Addr, Coin, Uint128};
    use injective_cosmwasm::MarketId;
    use pyth_sdk_cw::{testing::MockPyth, Price, PriceFeed, PriceIdentifier};

    use crate::{
        contract::execute,
        msg::{Cw20HookMsg, ExecuteMsg},
        state::{BasketAsset, Config, BASKET, CONFIG},
        tests::{
            setup_test, ATOMUSDT_MARKET_ID, INJUSDT_MARKET_ID, LP_TOKEN_ADDR, PRICE_ID_ATOM,
            PRICE_ID_INJ, PYTH_CONTRACT_ADDR, USDT,
        },
    };

    #[test]
    fn withdraw() {
        let current_unix_time = 10_000_000;
        let mut mock_pyth = MockPyth::new(Duration::from_secs(60), Coin::new(1, "foo"), &[]);
        let price_feed_inj = PriceFeed::new(
            PriceIdentifier::from_hex(PRICE_ID_INJ).unwrap(),
            Price {
                price: 900000000,
                conf: 10,
                expo: -8,
                publish_time: current_unix_time,
            },
            Price {
                price: 800000000,
                conf: 20,
                expo: -8,
                publish_time: current_unix_time,
            },
        );
        let price_feed_atom = PriceFeed::new(
            PriceIdentifier::from_hex(PRICE_ID_ATOM).unwrap(),
            Price {
                price: 1100000000,
                conf: 20,
                expo: -8,
                publish_time: current_unix_time,
            },
            Price {
                price: 1100000000,
                conf: 20,
                expo: -8,
                publish_time: current_unix_time,
            },
        );

        mock_pyth.add_feed(price_feed_inj);
        mock_pyth.add_feed(price_feed_atom);
        let (mut deps, env) = setup_test(&mock_pyth, current_unix_time);

        let mock_address = Addr::unchecked(LP_TOKEN_ADDR.to_owned());

        BASKET
            .save(
                &mut deps.storage,
                &crate::state::Basket {
                    assets: vec![
                        BasketAsset {
                            asset: Asset {
                                info: {
                                    AssetInfo::NativeToken {
                                        denom: String::from("inj"),
                                    }
                                },
                                amount: Uint128::zero(),
                            },
                            pyth_price_feed: PriceIdentifier::from_hex(PRICE_ID_INJ).unwrap(),
                            weight: Uint128::from(1u128),
                            spot_market_id: MarketId::new(INJUSDT_MARKET_ID).unwrap(),
                        },
                        BasketAsset {
                            asset: Asset {
                                info: {
                                    AssetInfo::NativeToken {
                                        denom: String::from("atom"),
                                    }
                                },
                                amount: Uint128::zero(),
                            },
                            pyth_price_feed: PriceIdentifier::from_hex(PRICE_ID_ATOM).unwrap(),
                            weight: Uint128::from(1u128),
                            spot_market_id: MarketId::new(ATOMUSDT_MARKET_ID).unwrap(),
                        },
                    ],
                },
            )
            .unwrap();

        CONFIG
            .save(
                &mut deps.storage,
                &Config {
                    lp_token: mock_address,
                    deposit_asset: AssetInfo::NativeToken {
                        denom: USDT.to_owned(),
                    },
                    pyth_contract_addr: Addr::unchecked(PYTH_CONTRACT_ADDR),
                },
            )
            .unwrap();

        let auth_info = mock_info(LP_TOKEN_ADDR, &coins(1, USDT.to_owned()));
        let msg: ExecuteMsg = ExecuteMsg::Receive(cw20::Cw20ReceiveMsg {
            sender: auth_info.sender.to_owned().into_string(),
            amount: Uint128::new(10),
            msg: to_binary(&Cw20HookMsg::Withdraw {}).unwrap(),
        });

        let res = execute(deps.as_mut(), env.to_owned(), auth_info, msg).unwrap();

        println!("{:?}", res.messages);
    }
}
