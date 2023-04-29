mod after_deposit;

pub use after_deposit::after_deposit;

use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, StdError};
use injective_cosmwasm::{InjectiveMsgWrapper, InjectiveQueryWrapper};

use crate::msg::CallbackMsg;

pub fn callback(
    deps: DepsMut<InjectiveQueryWrapper>,
    env: Env,
    info: MessageInfo,
    msg: CallbackMsg,
) -> Result<Response<InjectiveMsgWrapper>, StdError> {
    if info.sender != env.contract.address {
        return Err(StdError::GenericErr {
            msg: "Unauthorized".to_string(),
        });
    }

    match msg {
        CallbackMsg::AfterDeposit { deposit } => after_deposit(deps, env, deposit),
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use cosmwasm_std::{coins, testing::mock_info, Coin, Uint128};
    use pyth_sdk_cw::testing::MockPyth;

    use crate::{
        contract::execute,
        msg::{CallbackMsg, ExecuteMsg},
        state::DEPOSIT_PAID_CACHE,
        tests::{setup_test, CONTRACT_ADDR, USDT},
    };

    #[test]
    #[should_panic(expected = "Unauthorized")]
    fn unauthorized() {
        let current_unix_time = 10_000_000;
        let mock_pyth = MockPyth::new(Duration::from_secs(60), Coin::new(1, "foo"), &[]);
        let (mut deps, env) = setup_test(&mock_pyth, current_unix_time);

        let auth_info = mock_info("anyone", &coins(1, USDT.to_owned()));
        let msg = ExecuteMsg::Callback(CallbackMsg::AfterDeposit {
            deposit: Uint128::one(),
        });

        let _res = execute(deps.as_mut(), env.to_owned(), auth_info, msg).unwrap();
    }

    #[test]
    fn authorized() {
        let current_unix_time = 10_000_000;
        let mock_pyth = MockPyth::new(Duration::from_secs(60), Coin::new(1, "foo"), &[]);
        let (mut deps, env) = setup_test(&mock_pyth, current_unix_time);

        let auth_info = mock_info(CONTRACT_ADDR, &coins(1, USDT.to_owned()));
        let msg = ExecuteMsg::Callback(CallbackMsg::AfterDeposit {
            deposit: Uint128::one(),
        });

        DEPOSIT_PAID_CACHE
            .save(&mut deps.storage, &Uint128::zero())
            .unwrap();

        assert_eq!(
            execute(deps.as_mut(), env.to_owned(), auth_info, msg).is_ok(),
            true
        );
    }
}
