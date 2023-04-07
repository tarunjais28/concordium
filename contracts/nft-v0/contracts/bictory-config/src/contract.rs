use super::*;

/// Initialize the Config contract with Bictory's address.
#[init(contract = "BictoryConfig")]
fn contract_init(ctx: &impl HasInitContext) -> InitResult<State> {
    let owner = ctx.init_origin();

    // Initialising state
    let state = State::init(owner);

    Ok(state)
}

/// Takes target contract's contract address and function to send
/// AccountInfo details.
///
/// It rejects if:
/// - It fails to parse the parameter.
/// - Message sent back with the result rejects.
#[receive(
    contract = "BictoryConfig",
    name = "queryAccountInfo",
    parameter = "AccountInfoQueryParams"
)]
fn contract_query_account_info<A: HasActions>(
    ctx: &impl HasReceiveContext,
    state: &mut State,
) -> ContractResult<A> {
    // Parse the parameter.
    let params: AccountInfoQueryParams = ctx.parameter_cursor().get()?;

    // Send back the response.
    Ok(send(
        &params.result_contract,
        params.result_function.as_ref(),
        Amount::zero(),
        &state.account,
    ))
}

/// Transfer amount to account the
/// stored address.
///
/// It rejects if:
/// - It fails to parse the parameter.
/// - Message sent back with the result rejects.
#[receive(contract = "BictoryConfig", name = "sendCCD", payable)]
fn contract_send_ccd<A: HasActions>(
    _ctx: &impl HasReceiveContext,
    amount: Amount,
    state: &mut State,
) -> ContractResult<A> {
    // Send Bictory's share.
    Ok(A::simple_transfer(&state.account, amount))
}

/// Update address.
///
/// It rejects if:
/// - It fails to parse the parameter.
/// - Sender is other than state account.
/// - Fails to log `UpdateAddress` event.
#[receive(
    contract = "BictoryConfig",
    name = "updateAddress",
    parameter = "AccountAddress",
    enable_logger
)]
fn update_address<A: HasActions>(
    ctx: &impl HasReceiveContext,
    logger: &mut impl HasLogger,
    state: &mut State,
) -> ContractResult<A> {
    let address: AccountAddress = ctx.parameter_cursor().get()?;

    // Ensure the sender is the contract owner;
    ensure!(
        ctx.sender().matches_account(&state.account),
        ContractError::Unauthorized
    );

    state.account = address;

    // Event for update address.
    logger.log(&CustomEvent::UpdateAddress(address))?;

    Ok(A::accept())
}

#[concordium_cfg_test]
mod tests {
    use super::*;
    use test_infrastructure::*;

    const ACCOUNT_0: AccountAddress = AccountAddress([0u8; 32]);
    const ADDRESS_0: Address = Address::Account(ACCOUNT_0);
    const ACCOUNT_1: AccountAddress = AccountAddress([1u8; 32]);
    const ADDRESS_1: Address = Address::Account(ACCOUNT_1);

    /// Test set_account functionality
    #[concordium_test]
    fn test_init() {
        // Setup the context
        let mut ctx = InitContextTest::empty();
        ctx.set_init_origin(ACCOUNT_0);

        let royalty: u32 = 2_000_000;

        let parameter_bytes = to_bytes(&royalty);
        ctx.set_parameter(&parameter_bytes);

        let state = State::init(ACCOUNT_0);

        // Check the state
        claim!(state.account.eq(&ACCOUNT_0), "State not as expected");
    }

    /// Test set_account functionality
    #[concordium_test]
    fn test_update_address() {
        // Setup the context
        let mut ctx = ReceiveContextTest::empty();

        // Setting parameter
        let mut state = State::init(ACCOUNT_0);

        ctx.set_owner(ACCOUNT_0);
        ctx.set_sender(ADDRESS_0);
        let parameter_bytes = to_bytes(&ACCOUNT_1);
        ctx.set_parameter(&parameter_bytes);

        let mut logger = LogRecorder::init();

        // Call the contract function.
        let result: ContractResult<ActionsTree> = update_address(&ctx, &mut logger, &mut state);

        // Check the result
        let actions = result.expect_report("Results in rejection");
        claim_eq!(
            actions,
            ActionsTree::accept(),
            "No action should be produced."
        );

        claim_eq!(state.account, ACCOUNT_1, "ACCOUNT_1 is not updated.")
    }
}
