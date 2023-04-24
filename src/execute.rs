use astroport::asset::{Asset, AssetInfo};
use cosmwasm_std::{Addr, DepsMut, Env, MessageInfo, Response, StdError, StdResult};

use crate::{
    querier::query_token_info,
    query::basket_value,
    state::{BASKET, CONFIG},
    ContractError,
};

pub fn update_config(
    deps: DepsMut,
    _info: MessageInfo,
    _new_owner: Option<Addr>,
    _new_dens_addr: Option<Addr>,
) -> Result<Response, ContractError> {
    let _config = CONFIG.update(deps.storage, |config| -> Result<_, ContractError> {
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

    let _basket_value = basket_value(&deps.querier, env, &config, &basket)?;

    let liquidity_token = deps.api.addr_humanize(&config.lp_token)?;
    let _total_share = query_token_info(&deps.querier, liquidity_token)?.total_supply;

    Ok(Response::new().add_attribute("action", "deposit"))
}
