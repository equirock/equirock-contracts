use std::str::FromStr;
use std::time::Duration;

use crate::contract::{execute, instantiate, query};
use crate::msg::{ExecuteMsg, GetBasketAssetIdealRatioResponse, InstantiateMsg, QueryMsg};
use crate::state::{Basket, BasketAsset, Config, CONFIG};
use crate::ContractError;

use astroport::asset::{Asset, AssetInfo};
use cosmwasm_std::testing::{
    mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
};
use cosmwasm_std::{
    coin, coins, from_binary, to_binary, Addr, Api, Coin, Decimal, Env, OwnedDeps, QuerierResult,
    SystemError, SystemResult, Timestamp, Uint128, WasmQuery,
};
use cw20::TokenInfoResponse;
use pyth_sdk_cw::testing::MockPyth;
use pyth_sdk_cw::{Price, PriceFeed, PriceIdentifier, UnixTimestamp};

const PYTH_CONTRACT_ADDR: &str = "pyth_contract_addr";
const PRICE_ID_INJ: &str = "2d9315a88f3019f8efa88dfe9c0f0843712da0bac814461e27733f6b83eb51b3";
const PRICE_ID_ATOM: &str = "61226d39beea19d334f17c2febce27e12646d84675924ebb02b9cdaea68727e3";

const LP_TOKEN_ADDR: &str = "lp-token-0001";
const USDT: &str = "peggy0xdAC17F958D2ee523a2206206994597C13D831ec7";

fn setup_test(
    mock_pyth: &MockPyth,
    block_timestamp: UnixTimestamp,
) -> (OwnedDeps<MockStorage, MockApi, MockQuerier>, Env) {
    let mut dependencies = mock_dependencies();

    let mock_pyth_copy = (*mock_pyth).clone();
    dependencies
        .querier
        .update_wasm(move |x| handle_wasm_query(&mock_pyth_copy, x));

    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(u64::try_from(block_timestamp).unwrap());

    (dependencies, env)
}

fn handle_wasm_query(pyth: &MockPyth, wasm_query: &WasmQuery) -> QuerierResult {
    match wasm_query {
        WasmQuery::Smart { contract_addr, msg } if *contract_addr == PYTH_CONTRACT_ADDR => {
            pyth.handle_wasm_query(msg)
        }
        WasmQuery::Smart { contract_addr, msg } if *contract_addr == LP_TOKEN_ADDR => {
            SystemResult::Ok(
                to_binary(&TokenInfoResponse {
                    decimals: 6u8,
                    name: format!("LP"),
                    symbol: format!("LP"),
                    total_supply: Uint128::from(100u128),
                })
                .into(),
            )
        }
        WasmQuery::Smart { contract_addr, .. } => SystemResult::Err(SystemError::NoSuchContract {
            addr: contract_addr.clone(),
        }),
        WasmQuery::Raw { contract_addr, .. } => SystemResult::Err(SystemError::NoSuchContract {
            addr: contract_addr.clone(),
        }),
        WasmQuery::ContractInfo { contract_addr, .. } => {
            SystemResult::Err(SystemError::NoSuchContract {
                addr: contract_addr.clone(),
            })
        }
        _ => unreachable!(),
    }
}

#[test]
fn proper_initialization() {
    let current_unix_time = 10_000_000;
    let mock_pyth = MockPyth::new(Duration::from_secs(60), Coin::new(1, "foo"), &[]);
    let (mut deps, _env) = setup_test(&mock_pyth, current_unix_time);

    let msg = InstantiateMsg {
        etf_token_code_id: 1,
        etf_token_name: String::from("ER-Strategy-1"),
        deposit_asset: AssetInfo::NativeToken {
            denom: String::from("usdt"),
        },
        pyth_contract_addr: Addr::unchecked("pyth-contract-addr"),
        basket: Basket { assets: vec![] },
    };
    let info = mock_info("creator", &coins(1000, "earth"));

    let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(1, res.messages.len());

    let res = query(deps.as_ref(), mock_env(), QueryMsg::GetConfig {}).unwrap();
    let value: Config = from_binary(&res).unwrap();
}

#[test]
#[should_panic(expected = "Deposit other tokens")]
fn deposit_incorrect_denom() {
    let mut deps = mock_dependencies();

    let msg = InstantiateMsg {
        etf_token_code_id: 1,
        etf_token_name: String::from("ER-Strategy-1"),
        deposit_asset: AssetInfo::NativeToken {
            denom: String::from("usdt"),
        },
        pyth_contract_addr: Addr::unchecked("pyth-contract-addr"),
        basket: Basket { assets: vec![] },
    };
    let info = mock_info("creator", &vec![]);

    let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let auth_info = mock_info("anyone", &coins(1, "not-usdt"));
    let msg = ExecuteMsg::Deposit {
        asset: Asset {
            amount: Uint128::from(1u128),
            info: AssetInfo::NativeToken {
                denom: String::from("not-usdt"),
            },
        },
    };

    execute(deps.as_mut(), mock_env(), auth_info, msg).unwrap();
}

#[test]
#[should_panic(expected = "Native token balance mismatch between the argument and the transferred")]
fn deposit_different_than_in_asset() {
    let mut deps = mock_dependencies();

    let msg = InstantiateMsg {
        etf_token_code_id: 1,
        etf_token_name: String::from("ER-Strategy-1"),
        deposit_asset: AssetInfo::NativeToken {
            denom: String::from("usdt"),
        },
        pyth_contract_addr: Addr::unchecked("pyth-contract-addr"),
        basket: Basket { assets: vec![] },
    };
    let info = mock_info("creator", &vec![]);

    let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let auth_info = mock_info("anyone", &vec![coin(1, "usdt")]);
    let msg = ExecuteMsg::Deposit {
        asset: Asset {
            amount: Uint128::from(10u128),
            info: AssetInfo::NativeToken {
                denom: String::from("usdt"),
            },
        },
    };

    execute(deps.as_mut(), mock_env(), auth_info, msg).unwrap();
}

#[test]
fn deposit_correct_denom() {
    let current_unix_time = 10_000_000;
    let mock_pyth = MockPyth::new(Duration::from_secs(60), Coin::new(1, "foo"), &[]);
    let (mut deps, _env) = setup_test(&mock_pyth, current_unix_time);

    let msg = InstantiateMsg {
        etf_token_code_id: 1,
        etf_token_name: String::from("ER-Strategy-1"),
        deposit_asset: AssetInfo::NativeToken {
            denom: USDT.to_owned(),
        },
        pyth_contract_addr: Addr::unchecked("pyth-contract-addr"),
        basket: Basket { assets: vec![] },
    };
    let info = mock_info("creator", &vec![]);

    let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    CONFIG
        .update(
            &mut deps.storage,
            |mut config| -> Result<_, ContractError> {
                let mock_address = Addr::unchecked(LP_TOKEN_ADDR.to_owned());
                config.lp_token = deps.api.addr_canonicalize(&mock_address.as_str()).unwrap();
                Ok(config)
            },
        )
        .unwrap();

    let auth_info = mock_info("anyone", &coins(1, USDT.to_owned()));
    let msg = ExecuteMsg::Deposit {
        asset: Asset {
            amount: Uint128::from(1u128),
            info: AssetInfo::NativeToken {
                denom: USDT.to_owned(),
            },
        },
    };

    let res = execute(deps.as_mut(), mock_env(), auth_info, msg).unwrap();
}

#[test]
fn query_basket_prices() {
    let current_unix_time = 10_000_000;
    let mut mock_pyth = MockPyth::new(Duration::from_secs(60), Coin::new(1, "foo"), &[]);
    let price_feed_inj = PriceFeed::new(
        PriceIdentifier::from_hex(PRICE_ID_INJ).unwrap(),
        Price {
            price: 100,
            conf: 10,
            expo: -1,
            publish_time: current_unix_time,
        },
        Price {
            price: 80,
            conf: 20,
            expo: -1,
            publish_time: current_unix_time,
        },
    );
    let price_feed_atom = PriceFeed::new(
        PriceIdentifier::from_hex(PRICE_ID_ATOM).unwrap(),
        Price {
            price: 200,
            conf: 20,
            expo: -1,
            publish_time: current_unix_time,
        },
        Price {
            price: 110,
            conf: 20,
            expo: -1,
            publish_time: current_unix_time,
        },
    );

    mock_pyth.add_feed(price_feed_inj);
    mock_pyth.add_feed(price_feed_atom);

    let (mut deps, env) = setup_test(&mock_pyth, current_unix_time);

    let msg = InstantiateMsg {
        etf_token_code_id: 1,
        etf_token_name: String::from("ER-Strategy-1"),
        deposit_asset: AssetInfo::NativeToken {
            denom: String::from("usdt"),
        },
        pyth_contract_addr: Addr::unchecked(PYTH_CONTRACT_ADDR),
        basket: Basket {
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
                },
            ],
        },
    };
    let info = mock_info("creator", &vec![]);

    let res = instantiate(deps.as_mut(), env.to_owned(), info, msg).unwrap();

    assert_eq!(1, res.messages.len());

    let res = query(
        deps.as_ref(),
        env.to_owned(),
        QueryMsg::GetBasketIdealRatio {},
    )
    .unwrap();
    let value: Vec<GetBasketAssetIdealRatioResponse> = from_binary(&res).unwrap();

    assert_eq!(
        value.into_iter().map(|a| a.ratio).collect::<Vec<Decimal>>(),
        vec![
            Decimal::from_str("0.05").unwrap(),
            Decimal::from_str("0.025").unwrap()
        ]
    );
}
