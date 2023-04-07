use super::*;

pub fn get_update_operator_action<S: HasStateApi>(
    host: &mut impl HasHost<State<S>, StateApiType = S>,
    self_address: ContractAddress,
    target_contract: &ContractAddress,
    update_type: OperatorUpdate,
) -> ContractResult<()> {
    let update_operator: UpdateOperatorParams = UpdateOperatorParams(vec![UpdateOperator {
        update: update_type,
        operator: Address::Contract(self_address),
    }]);
    let entrypoint_name = EntrypointName::new_unchecked("updateOperator");

    host.invoke_contract(
        target_contract,
        &update_operator,
        entrypoint_name,
        Amount::zero(),
    )?;

    Ok(())
}

pub fn get_update_price_action<S: HasStateApi>(
    host: &mut impl HasHost<State<S>, StateApiType = S>,
    token: &Token,
    price: Amount,
) -> ContractResult<()> {
    let update_price = UpdatePriceParameter {
        token_id: token.id.clone(),
        price,
    };
    let entrypoint_name = EntrypointName::new_unchecked("updatePrice");

    host.invoke_contract(
        &token.contract,
        &update_price,
        entrypoint_name,
        Amount::zero(),
    )?;

    Ok(())
}

pub fn get_account_address(address: Address) -> ContractResult<AccountAddress> {
    match address {
        Address::Account(addr) => Ok(addr),
        Address::Contract(_) => bail!(CustomContractError::OnlyAccountAddress.into()),
    }
}
