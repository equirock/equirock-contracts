use std::marker::PhantomData;
use std::str::FromStr;
use std::time::Duration;

use crate::contract::{execute, instantiate, query};
use crate::helpers::get_message_data;
use crate::msg::{ExecuteMsg, GetBasketAssetIdealRatioResponse, InstantiateMsg, QueryMsg};
use crate::state::{Basket, BasketAsset, Config, CONFIG};
use crate::ContractError;

use astroport::asset::{Asset, AssetInfo};
use cosmwasm_std::testing::{mock_env, mock_info, MockApi, MockStorage};
use cosmwasm_std::{
    coin, coins, from_binary, to_binary, Addr, Api, Coin, ContractResult, Decimal, Env, OwnedDeps,
    QuerierResult, SystemError, SystemResult, Timestamp, Uint128, WasmQuery,
};
use cw20::TokenInfoResponse;
use injective_cosmwasm::{
    mock_dependencies, DenomDecimals, HandlesDenomDecimalsQuery, HandlesMarketIdQuery,
    HandlesSmartQuery, InjectiveMsg, InjectiveQueryWrapper, MarketId, QueryDenomDecimalsResponse,
    SpotMarket, SpotMarketResponse, WasmMockQuerier,
};
use injective_math::FPDecimal;
use pyth_sdk_cw::testing::MockPyth;
use pyth_sdk_cw::{Price, PriceFeed, PriceIdentifier, UnixTimestamp};

const PYTH_CONTRACT_ADDR: &str = "pyth_contract_addr";
const PRICE_ID_INJ: &str = "2d9315a88f3019f8efa88dfe9c0f0843712da0bac814461e27733f6b83eb51b3";
const PRICE_ID_ATOM: &str = "61226d39beea19d334f17c2febce27e12646d84675924ebb02b9cdaea68727e3";

pub const LP_TOKEN_ADDR: &str = "lp-token-0001";
pub const USDT: &str = "peggy0xdAC17F958D2ee523a2206206994597C13D831ec7";
pub const ATOM: &str = "factory/inj17vytdwqczqz72j65saukplrktd4gyfme5agf6c/atom";

const INJUSDT_MARKET_ID: &str =
    "0x0611780ba69656949525013d947713300f56c37b6175e02f26bffa495c3208fe";
const ATOMUSDT_MARKET_ID: &str =
    "0x491ee4fae7956dd72b6a97805046ffef65892e1d3254c559c18056a519b2ca15";

pub const CONTRACT_ADDR: &str = "inj1qge3zfgncdyssvqhl7az3gh93q7sqffm4rje87";

pub fn inj_mock_deps(
    mock_pyth: &MockPyth,
) -> OwnedDeps<MockStorage, MockApi, WasmMockQuerier, InjectiveQueryWrapper> {
    let mut custom_querier: WasmMockQuerier = WasmMockQuerier::new();
    custom_querier.smart_query_handler = Some(Box::new(create_smart_query_handler(mock_pyth)));
    custom_querier.spot_market_response_handler = Some(Box::new(create_spot_market_handler()));
    custom_querier.denom_decimals_handler = Some(Box::new(create_denom_decimals_handler()));
    OwnedDeps {
        api: MockApi::default(),
        storage: MockStorage::default(),
        querier: custom_querier,
        custom_query_type: PhantomData::default(),
    }
}

fn create_smart_query_handler(mock_pyth: &MockPyth) -> impl HandlesSmartQuery {
    struct Temp {
        mock_pyth: MockPyth,
    }
    impl HandlesSmartQuery for Temp {
        fn handle(&self, contract_addr: &str, msg: &cosmwasm_std::Binary) -> QuerierResult {
            if contract_addr == PYTH_CONTRACT_ADDR {
                return self.mock_pyth.handle_wasm_query(msg);
            }

            if contract_addr == LP_TOKEN_ADDR {
                return SystemResult::Ok(
                    to_binary(&TokenInfoResponse {
                        decimals: 6u8,
                        name: format!("LP"),
                        symbol: format!("LP"),
                        total_supply: Uint128::from(100u128),
                    })
                    .into(),
                );
            }

            SystemResult::Err(SystemError::NoSuchContract {
                addr: contract_addr.to_owned(),
            })
        }
    }
    Temp {
        mock_pyth: mock_pyth.to_owned(),
    }
}

fn create_spot_market_handler() -> impl HandlesMarketIdQuery {
    struct Temp {}
    impl HandlesMarketIdQuery for Temp {
        fn handle(&self, market_id: MarketId) -> QuerierResult {
            if market_id.as_str() == ATOMUSDT_MARKET_ID {
                let response = SpotMarketResponse {
                    market: Some(SpotMarket {
                        ticker: ATOM.to_string(),
                        base_denom: "factory/inj17vytdwqczqz72j65saukplrktd4gyfme5agf6c/atom"
                            .to_string(),
                        quote_denom: USDT.to_string(),
                        maker_fee_rate: FPDecimal::from_str("0.001").unwrap(),
                        taker_fee_rate: FPDecimal::from_str("0.002").unwrap(),
                        relayer_fee_share_rate: FPDecimal::from_str("0.4").unwrap(),
                        market_id,
                        status: 0,
                        min_price_tick_size: FPDecimal::from_str("0.0010000000000000").unwrap(),
                        min_quantity_tick_size: FPDecimal::from_str("1000.000000000000000000")
                            .unwrap(),
                    }),
                };
                return SystemResult::Ok(ContractResult::from(to_binary(&response)));
            }

            let response = SpotMarketResponse {
                market: Some(SpotMarket {
                    ticker: "INJ/USDT".to_string(),
                    base_denom: "inj".to_string(),
                    quote_denom: USDT.to_string(),
                    maker_fee_rate: FPDecimal::from_str("0.001").unwrap(),
                    taker_fee_rate: FPDecimal::from_str("0.002").unwrap(),
                    relayer_fee_share_rate: FPDecimal::from_str("0.4").unwrap(),
                    market_id,
                    status: 0,
                    min_price_tick_size: FPDecimal::from_str("0.000000000000001000").unwrap(),
                    min_quantity_tick_size: FPDecimal::from_str(
                        "1000000000000000.000000000000000000",
                    )
                    .unwrap(),
                }),
            };
            SystemResult::Ok(ContractResult::from(to_binary(&response)))
        }
    }
    Temp {}
}

fn create_denom_decimals_handler() -> impl HandlesDenomDecimalsQuery {
    struct Temp {}
    impl HandlesDenomDecimalsQuery for Temp {
        fn handle(&self, denoms: Vec<String>) -> QuerierResult {
            let response = QueryDenomDecimalsResponse {
                denom_decimals: denoms
                    .iter()
                    .map(|d| DenomDecimals {
                        decimals: match d.as_str() {
                            "inj" => 18u64,
                            ATOM => 6u64,
                            USDT => 6u64,
                            _ => 18u64,
                        },
                        denom: d.to_owned(),
                    })
                    .collect(),
            };
            SystemResult::Ok(ContractResult::from(to_binary(&response)))
        }
    }
    Temp {}
}

pub fn setup_test(
    mock_pyth: &MockPyth,
    block_timestamp: UnixTimestamp,
) -> (
    OwnedDeps<MockStorage, MockApi, WasmMockQuerier, InjectiveQueryWrapper>,
    Env,
) {
    let dependencies = inj_mock_deps(mock_pyth);

    // let mock_pyth_copy = (*mock_pyth).clone();
    // dependencies
    //     .querier
    //     .update_wasm(move |x| handle_wasm_query(&mock_pyth_copy, x));

    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(u64::try_from(block_timestamp).unwrap());
    env.contract.address = Addr::unchecked(CONTRACT_ADDR);

    (dependencies, env)
}

fn _handle_wasm_query(pyth: &MockPyth, wasm_query: &WasmQuery) -> QuerierResult {
    match wasm_query {
        WasmQuery::Smart { contract_addr, msg } if *contract_addr == PYTH_CONTRACT_ADDR => {
            pyth.handle_wasm_query(msg)
        }
        WasmQuery::Smart {
            contract_addr,
            msg: _,
        } if *contract_addr == LP_TOKEN_ADDR => SystemResult::Ok(
            to_binary(&TokenInfoResponse {
                decimals: 6u8,
                name: format!("LP"),
                symbol: format!("LP"),
                total_supply: Uint128::from(100u128),
            })
            .into(),
        ),
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
    let _value: Config = from_binary(&res).unwrap();
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

    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

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

    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

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
    let (mut deps, env) = setup_test(&mock_pyth, current_unix_time);

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

    let _res = instantiate(deps.as_mut(), env.to_owned(), info, msg).unwrap();

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

    let _res = execute(deps.as_mut(), env.to_owned(), auth_info, msg).unwrap();
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
                    spot_market_id: MarketId::new(INJUSDT_MARKET_ID).unwrap(),
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

#[test]
fn deposit() {
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

    let msg = InstantiateMsg {
        etf_token_code_id: 1,
        etf_token_name: String::from("ER-Strategy-1"),
        deposit_asset: AssetInfo::NativeToken {
            denom: String::from(USDT),
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
    };
    let info = mock_info("creator", &vec![]);

    let _res = instantiate(deps.as_mut(), env.to_owned(), info, msg).unwrap();

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

    let asset = Asset {
        amount: Uint128::from(1_000_000u128),
        info: AssetInfo::NativeToken {
            denom: String::from(USDT),
        },
    };
    let info = mock_info(
        "creator",
        &vec![Coin {
            amount: asset.amount.to_owned(),
            denom: String::from(USDT),
        }],
    );

    let msg = ExecuteMsg::Deposit { asset: asset };

    let res = execute(deps.as_mut(), env.to_owned(), info, msg).unwrap();
    let messages = res.messages;

    // res.messages.len()
    assert_eq!(messages.len(), 3);

    if let InjectiveMsg::CreateSpotMarketOrder { sender, order } =
        &get_message_data(&messages, 0).msg_data
    {
        assert_eq!(sender.to_string(), CONTRACT_ADDR, "sender not correct");
        assert_eq!(order.market_id.as_str(), INJUSDT_MARKET_ID);
        assert_eq!(
            order.order_info.quantity.to_string(),
            format!("55000000000000000")
        );
        assert_eq!(
            order.order_info.price.to_string(),
            format!("0.00000000000909")
        );
    } else {
        panic!("Wrong message type!");
    }

    if let InjectiveMsg::CreateSpotMarketOrder { sender, order } =
        &get_message_data(&messages, 1).msg_data
    {
        assert_eq!(sender.to_string(), CONTRACT_ADDR, "sender not correct");
        assert_eq!(order.market_id.as_str(), ATOMUSDT_MARKET_ID);
        assert_eq!(order.order_info.quantity.to_string(), format!("45000"));
        assert_eq!(order.order_info.price.to_string(), format!("11.11"));
    } else {
        panic!("Wrong message type!");
    }
}

#[test]
fn query_basket_value() {
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

    let msg = InstantiateMsg {
        etf_token_code_id: 1,
        etf_token_name: String::from("ER-Strategy-1"),
        deposit_asset: AssetInfo::NativeToken {
            denom: String::from(USDT),
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
    };
    let info = mock_info("creator", &vec![]);

    let _res = instantiate(deps.as_mut(), env.to_owned(), info, msg).unwrap();

    let res = query(deps.as_ref(), env, QueryMsg::GetBasketValueInUsdt {}).unwrap();
    let value: Uint128 = from_binary(&res).unwrap();
}
