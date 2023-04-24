#[cfg(test)]
mod tests {
    use crate::helpers::CwTemplateContract;
    use crate::msg::InstantiateMsg;
    use crate::state::Basket;
    use astroport::asset::AssetInfo;
    use cosmwasm_std::{Addr, Coin, Empty, Uint128};
    use cw_multi_test::{App, AppBuilder, Contract, ContractWrapper, Executor};

    const PYTH_CONTRACT_ADDR: &str = "pyth_contract_addr";

    pub fn contract_template() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        )
        .with_reply(crate::contract::reply);
        Box::new(contract)
    }

    pub fn cw20_template() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            cw20_base::contract::execute,
            cw20_base::contract::instantiate,
            cw20_base::contract::query,
        );
        Box::new(contract)
    }

    const USER: &str = "USER";
    const ADMIN: &str = "ADMIN";
    const NATIVE_DENOM: &str = "denom";

    fn mock_app() -> App {
        AppBuilder::new().build(|router, _, storage| {
            router
                .bank
                .init_balance(
                    storage,
                    &Addr::unchecked(USER),
                    vec![Coin {
                        denom: NATIVE_DENOM.to_string(),
                        amount: Uint128::new(1),
                    }],
                )
                .unwrap();
        })
    }

    fn proper_instantiate() -> (App, CwTemplateContract) {
        let mut app = mock_app();
        let equirock_template_id = app.store_code(contract_template());
        let cw20_code_id = app.store_code(cw20_template());

        let msg = InstantiateMsg {
            etf_token_code_id: cw20_code_id,
            etf_token_name: String::from("ER-Strategy-1"),
            deposit_asset: AssetInfo::NativeToken {
                denom: String::from("usdt"),
            },
            basket: Basket { assets: vec![] },
            pyth_contract_addr: Addr::unchecked(PYTH_CONTRACT_ADDR),
        };
        let cw_template_contract_addr = app
            .instantiate_contract(
                equirock_template_id,
                Addr::unchecked(ADMIN),
                &msg,
                &[],
                "test",
                None,
            )
            .unwrap();

        let cw_template_contract = CwTemplateContract(cw_template_contract_addr);

        (app, cw_template_contract)
    }

    mod update_config {
        use super::*;
        use crate::msg::ExecuteMsg;

        #[test]
        fn update_config() {
            let (mut app, cw_template_contract) = proper_instantiate();

            let msg = ExecuteMsg::UpdateConfig {};
            let cosmos_msg = cw_template_contract.call(msg).unwrap();
            let _res = app.execute(Addr::unchecked(USER), cosmos_msg.clone());

            app.execute(Addr::unchecked(ADMIN), cosmos_msg).unwrap();
        }
    }
}
