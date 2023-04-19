#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, CanonicalAddr, Decimal, Deps, DepsMut, Env, MessageInfo, Reply,
    ReplyOn, Response, StdError, StdResult, SubMsg, WasmMsg,
};
use cw2::set_contract_version;
use cw20_base::msg::InstantiateMsg as CW20InstantiateMsg;
use protobuf::Message;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::{Asset, AssetInfo, Config, CONFIG};

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

    let config = Config {
        lp_token: CanonicalAddr::from(vec![]),
        deposit_asset: msg.deposit_asset,
    };

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    CONFIG.save(deps.storage, &config)?;

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
    use cosmwasm_std::{Addr, QueryRequest, WasmQuery};
    use cw721::{Cw721QueryMsg, OwnerOfResponse};

    use crate::state::CONFIG;

    use super::*;

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

        Ok(Response::new().add_attribute("action", "deposit"))
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig {} => to_binary(&query::config(deps)?),
    }
}

pub mod query {

    use crate::state::{Config, CONFIG};

    use super::*;

    pub fn config(deps: Deps) -> StdResult<Config> {
        let config = CONFIG.load(deps.storage)?;
        Ok(config)
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

#[cfg(test)]
mod tests {

    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coin, coins, from_binary, Uint128};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            etf_token_code_id: 1,
            etf_token_name: String::from("ER-Strategy-1"),
            deposit_asset: AssetInfo::NativeToken {
                denom: String::from("usdt"),
            },
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
    #[should_panic(expected = "Native token balance mismatch")]
    fn deposit_different_than_in_asset() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            etf_token_code_id: 1,
            etf_token_name: String::from("ER-Strategy-1"),
            deposit_asset: AssetInfo::NativeToken {
                denom: String::from("usdt"),
            },
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
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            etf_token_code_id: 1,
            etf_token_name: String::from("ER-Strategy-1"),
            deposit_asset: AssetInfo::NativeToken {
                denom: String::from("usdt"),
            },
        };
        let info = mock_info("creator", &vec![]);

        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let auth_info = mock_info("anyone", &coins(1, "usdt"));
        let msg = ExecuteMsg::Deposit {
            asset: Asset {
                amount: Uint128::from(1u128),
                info: AssetInfo::NativeToken {
                    denom: String::from("usdt"),
                },
            },
        };

        let res = execute(deps.as_mut(), mock_env(), auth_info, msg).unwrap();
    }
}
