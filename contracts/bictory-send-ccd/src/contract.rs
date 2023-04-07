use super::*;

/// Initialize the listing contract with an empty list of listings.
#[init(contract = "BictorySendCCD")]
fn init<S: HasStateApi>(
    _ctx: &impl HasInitContext,
    _state_builder: &mut StateBuilder<S>,
) -> InitResult<State<S>> {
    Ok(State::empty())
}

/// Send CCD to destination account.
/// Takes destination address as a parameter.
///
/// Rejects if:
/// - It fails to parse the parameter.
/// - It fails to log `SendCCDEvent`.
#[receive(
    contract = "BictorySendCCD",
    name = "send",
    parameter = "AccountAddress",
    mutable,
    enable_logger,
    payable
)]
fn send<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
    amount: Amount,
    logger: &mut impl HasLogger,
) -> ContractResult<()> {
    let dest_addr: AccountAddress = ctx.parameter_cursor().get()?;

    // Event for sending CCD.
    logger.log(&CustomEvent::Send(SendCCDEvent {
        account: dest_addr,
        amount,
    }))?;

    host.invoke_transfer(&dest_addr, amount)?;

    Ok(())
}

#[concordium_cfg_test]
mod tests {
    use super::*;
    use test_infrastructure::*;

    /// Test initialization succeeds.
    #[concordium_test]
    fn test_init() {
        // Setup the context
        let ctx = TestInitContext::empty();
        let mut state_builder = TestStateBuilder::new();

        // Call the contract function.
        let result = init(&ctx, &mut state_builder);

        // Check the state
        claim!(result.is_ok(), "State should remain intacted");
    }
}
