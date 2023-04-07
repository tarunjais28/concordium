use super::*;

/// Initialize the listing contract with an empty list of listings.
#[init(contract = "BictorySendCCD")]
fn contract_init(_ctx: &impl HasInitContext) -> InitResult<State> {
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
    enable_logger,
    payable
)]
fn contract_buy<A: HasActions>(
    ctx: &impl HasReceiveContext,
    amount: Amount,
    logger: &mut impl HasLogger,
    _state: &mut State,
) -> ContractResult<A> {
    let dest_addr: AccountAddress = ctx.parameter_cursor().get()?;

    // Event for buying NFT.
    logger.log(&CustomEvent::Send(SendCCDEvent {
        account: dest_addr,
        amount,
    }))?;

    Ok(A::simple_transfer(&dest_addr, amount))
}

#[concordium_cfg_test]
mod tests {
    use super::*;
    use test_infrastructure::*;

    /// Test initialization succeeds.
    #[concordium_test]
    fn test_init() {
        // Setup the context
        let ctx = InitContextTest::empty();

        // Call the contract function.
        let result = contract_init(&ctx);

        // Check the result
        let state = result.expect_report("Contract initialization failed");

        // Check the state
        claim_eq!(state.send, SendCCD::Send, "State should remain intacted");
    }
}
