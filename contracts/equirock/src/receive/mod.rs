use cosmwasm_std::{from_binary, DepsMut, Env, MessageInfo, Response, StdError, StdResult};

mod withdraw;

use cw20::Cw20ReceiveMsg;
use injective_cosmwasm::{InjectiveMsgWrapper, InjectiveQueryWrapper};
use withdraw::withdraw;

use crate::{msg::Cw20HookMsg, state::CONFIG};

pub fn receive(
    deps: DepsMut<InjectiveQueryWrapper>,
    env: Env,
    info: MessageInfo,
    msg: Cw20ReceiveMsg,
) -> StdResult<Response<InjectiveMsgWrapper>> {
    let config = CONFIG.load(deps.storage)?;

    if info.sender != config.lp_token {
        return Err(StdError::GenericErr {
            msg: "Unauthorized".to_string(),
        });
    }

    match from_binary(&msg.msg)? {
        Cw20HookMsg::Withdraw {} => withdraw(deps, env, msg.sender, msg.amount),
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use astroport::asset::AssetInfo;
    use cosmwasm_std::{coins, testing::mock_info, to_binary, Addr, Coin, Uint128};
    use pyth_sdk_cw::testing::MockPyth;

    use crate::{
        contract::execute,
        msg::{Cw20HookMsg, ExecuteMsg},
        state::{Config, BASKET, CONFIG},
        tests::{setup_test, LP_TOKEN_ADDR, USDT},
    };

    #[test]
    #[should_panic(expected = "Unauthorized")]
    fn unauthorized() {
        let current_unix_time = 10_000_000;
        let mock_pyth = MockPyth::new(Duration::from_secs(60), Coin::new(1, "foo"), &[]);
        let (mut deps, env) = setup_test(&mock_pyth, current_unix_time);

        let mock_address = Addr::unchecked(LP_TOKEN_ADDR.to_owned());

        CONFIG
            .save(
                &mut deps.storage,
                &Config {
                    lp_token: mock_address,
                    deposit_asset: AssetInfo::NativeToken {
                        denom: USDT.to_owned(),
                    },
                    pyth_contract_addr: Addr::unchecked("pyth-contract-addr"),
                },
            )
            .unwrap();

        let auth_info = mock_info("anyone", &coins(1, USDT.to_owned()));
        let msg: ExecuteMsg = ExecuteMsg::Receive(cw20::Cw20ReceiveMsg {
            sender: auth_info.sender.to_owned().into_string(),
            amount: Uint128::new(1_000),
            msg: to_binary(&Cw20HookMsg::Withdraw {}).unwrap(),
        });

        let _res = execute(deps.as_mut(), env.to_owned(), auth_info, msg).unwrap();
    }

    #[test]
    fn authorized() {
        let current_unix_time = 10_000_000;
        let mock_pyth = MockPyth::new(Duration::from_secs(60), Coin::new(1, "foo"), &[]);
        let (mut deps, env) = setup_test(&mock_pyth, current_unix_time);

        let mock_address = Addr::unchecked(LP_TOKEN_ADDR.to_owned());

        BASKET
            .save(&mut deps.storage, &crate::state::Basket { assets: vec![] })
            .unwrap();

        CONFIG
            .save(
                &mut deps.storage,
                &Config {
                    lp_token: mock_address,
                    deposit_asset: AssetInfo::NativeToken {
                        denom: USDT.to_owned(),
                    },
                    pyth_contract_addr: Addr::unchecked("pyth-contract-addr"),
                },
            )
            .unwrap();

        let auth_info = mock_info(LP_TOKEN_ADDR, &coins(1, USDT.to_owned()));
        let msg: ExecuteMsg = ExecuteMsg::Receive(cw20::Cw20ReceiveMsg {
            sender: auth_info.sender.to_owned().into_string(),
            amount: Uint128::new(1_000),
            msg: to_binary(&Cw20HookMsg::Withdraw {}).unwrap(),
        });

        let _res = execute(deps.as_mut(), env.to_owned(), auth_info, msg).unwrap();
    }
}
