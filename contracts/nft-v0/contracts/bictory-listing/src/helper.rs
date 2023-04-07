use super::*;

pub fn get_update_operator_action<A: HasActions>(
    self_address: ContractAddress,
    target_contract: &ContractAddress,
    update_type: OperatorUpdate,
) -> A {
    let update_operator: UpdateOperatorParams = UpdateOperatorParams(vec![UpdateOperator {
        update: update_type,
        operator: Address::Contract(self_address),
    }]);
    let receive_name = ReceiveName::new_unchecked("BictoryNFT.updateOperator");

    send(
        target_contract,
        receive_name,
        Amount::zero(),
        &update_operator,
    )
}

pub fn get_update_price_action<A: HasActions>(token: &Token, price: Amount) -> A {
    let update_price = UpdatePriceParameter {
        token_id: token.id.clone(),
        price,
    };
    let receive_name = ReceiveName::new_unchecked("BictoryNFT.updatePrice");

    send(&token.contract, receive_name, Amount::zero(), &update_price)
}
