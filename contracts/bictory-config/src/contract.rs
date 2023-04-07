use super::*;

/// Initialize the Config contract with Bictory's address.
#[init(contract = "BictoryConfig")]
fn init<S: HasStateApi>(
    ctx: &impl HasInitContext,
    _state_builder: &mut StateBuilder<S>,
) -> InitResult<State<S>> {
    let owner = ctx.init_origin();

    // Initialising state
    let state = State::init(owner);

    Ok(state)
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
    mutable,
    enable_logger
)]
fn update_address<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
    logger: &mut impl HasLogger,
) -> ContractResult<()> {
    let address: AccountAddress = ctx.parameter_cursor().get()?;
    let state = host.state_mut();

    // Ensure the sender is the contract owner;
    ensure!(
        ctx.sender().matches_account(&state.account),
        ContractError::Unauthorized
    );

    state.update_address(address);

    // Event for update address.
    logger.log(&CustomEvent::UpdateAddress(address))?;

    Ok(())
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
    parameter = "AccountInfoQueryParams",
    mutable
)]
fn query_account_info<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<()> {
    // Parse the parameter.
    let params: AccountInfoQueryParams = ctx.parameter_cursor().get()?;
    let address = &host.state_mut().account.clone();

    // Send back the response.
    host.invoke_contract(
        &params.result_contract,
        address,
        params.result_function.as_receive_name().entrypoint_name(),
        Amount::zero(),
    )?;

    Ok(())
}

/// Takes royalty as input and transfer amount to account the
/// stored address.
///
/// It rejects if:
/// - It fails to parse the parameter.
/// - Message sent back with the result rejects.
/// - Mismatch in source and target royalties.
#[receive(contract = "BictoryConfig", name = "sendCCD", mutable, payable)]
fn send_ccd<S: HasStateApi>(
    _ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
    amount: Amount,
) -> ContractResult<()> {
    let state = host.state();

    // Send Bictory's share.
    host.invoke_transfer(&state.account, amount)?;

    Ok(())
}

/// View function that returns address of bictory's wallet
#[receive(
    contract = "BictoryConfig",
    name = "view",
    return_value = "AccountAddress"
)]
fn view<S: HasStateApi>(
    _ctx: &impl HasReceiveContext,
    host: &impl HasHost<State<S>, StateApiType = S>,
) -> ReceiveResult<AccountAddress> {
    let state = host.state();
    
    Ok(state.account)
}

#[concordium_cfg_test]
mod tests {
    use super::*;
    use test_infrastructure::*;

    const ACCOUNT_0: AccountAddress = AccountAddress([0u8; 32]);
    const ADDRESS_0: Address = Address::Account(ACCOUNT_0);
    const ACCOUNT_1: AccountAddress = AccountAddress([1u8; 32]);

    /// Test set_account functionality
    #[concordium_test]
    fn test_init() {
        // Setup the context
        let mut ctx = TestInitContext::empty();
        ctx.set_init_origin(ACCOUNT_0);

        let royalty: u32 = 2_000_000;

        let parameter_bytes = to_bytes(&royalty);
        ctx.set_parameter(&parameter_bytes);

        let state: State<TestStateApi> = State::init(ACCOUNT_0);

        // Check the state
        claim!(state.account == ACCOUNT_0, "State not as expected");
    }

    /// Test update address functionality
    #[concordium_test]
    fn test_update_address() {
        // Setup the context
        let mut ctx = TestReceiveContext::empty();

        // Setting parameter
        let state = State::init(ACCOUNT_0);
        let state_builder = TestStateBuilder::new();
        let mut host = TestHost::new(state, state_builder);

        ctx.set_owner(ACCOUNT_0);
        ctx.set_sender(ADDRESS_0);
        let parameter_bytes = to_bytes(&ACCOUNT_1);
        ctx.set_parameter(&parameter_bytes);

        let mut logger = TestLogger::init();

        // Call the contract function.
        let result: ContractResult<()> = update_address(&ctx, &mut host, &mut logger);

        // Check the result
        claim!(result.is_ok(), "Results in rejection");

        claim_eq!(host.state().account, ACCOUNT_1, "ACCOUNT_1 is not updated.")
    }
}
