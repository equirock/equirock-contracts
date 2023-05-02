use astroport::asset::AssetInfo;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Reply, ReplyOn, Response, StdError,
    StdResult, SubMsg, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw20_base::msg::InstantiateMsg as CW20InstantiateMsg;
use injective_cosmwasm::{InjectiveMsgWrapper, InjectiveQueryWrapper};

use crate::callback::callback;
use crate::error::ContractError;
use crate::execute::{deposit, update_config};
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::query::{config, get_basket_ideal_ratio, get_basket_value};
use crate::receive::receive;
use crate::reply::{handle_lp_init, handle_order, ATOMIC_ORDER_REPLY_ID, INSTANTIATE_REPLY_ID};
use crate::state::{Config, BASKET, CONFIG};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:equirock-contract";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut<InjectiveQueryWrapper>,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response<InjectiveMsgWrapper>, ContractError> {
    if let AssetInfo::Token { .. } = &msg.deposit_asset {
        return Err(StdError::generic_err("Tokens not supported").into());
    }

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        lp_token: Addr::unchecked(""),
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
    deps: DepsMut<InjectiveQueryWrapper>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<InjectiveMsgWrapper>, StdError> {
    match msg {
        ExecuteMsg::UpdateConfig {} => update_config(deps, info, None, None),
        ExecuteMsg::Deposit { asset } => deposit(deps, env, info, asset),
        ExecuteMsg::Callback(msg) => callback(deps, env, info, msg),
        ExecuteMsg::Receive(msg) => receive(deps, env, info, msg),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps<InjectiveQueryWrapper>, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig {} => to_binary(&config(deps)?),
        QueryMsg::GetBasketIdealRatio {} => to_binary(&get_basket_ideal_ratio(deps, &env)?),
        QueryMsg::GetBasketValueInUsdt {} => to_binary(&get_basket_value(deps, &env)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(
    deps: DepsMut,
    env: Env,
    msg: Reply,
) -> Result<Response<InjectiveMsgWrapper>, ContractError> {
    match msg.id {
        INSTANTIATE_REPLY_ID => handle_lp_init(deps, env, msg),
        ATOMIC_ORDER_REPLY_ID => handle_order(deps, env, msg),
        _ => Err(ContractError::UnrecognisedReply(msg.id)),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}
