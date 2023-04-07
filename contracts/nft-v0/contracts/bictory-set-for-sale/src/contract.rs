use super::*;

/// Initialize the SetForSale contract with an empty list of listings.
#[init(contract = "BictorySetForSale")]
fn contract_init(_ctx: &impl HasInitContext) -> InitResult<State> {
    Ok(State::empty())
}

/// Enable or Disable for_sale value to put contract on sale or remove from sale.
/// Can only be called by the token owner.
/// Logs a `SetForSale`.
///
/// It rejects if:
/// - The sender is not the token owner.
/// - Fails to parse parameter.
/// - Tokens fails to be upated, which could be if:
///     - Fails to log SetForSale event
#[receive(
    contract = "BictorySetForSale",
    name = "setForSale",
    parameter = "Sales",
    enable_logger
)]
pub fn contract_set_for_sale<A: HasActions>(
    ctx: &impl HasReceiveContext,
    logger: &mut impl HasLogger,
    state: &mut State,
) -> ContractResult<A> {
    let sender = ctx.sender();
    // Parse the parameter.
    let params: Sales = ctx.parameter_cursor().get()?;
    let mut actions = A::accept();

    // Ensuring only owner of NFT can list
    ensure!(
        sender.matches_account(&params.owner),
        CustomContractError::OnlyOwner.into()
    );

    for sales in params.sales {
        // Updating state
        state.update_state(params.owner, &sales)?;

        // Burning Token from the target contract
        let mut receive_name = ReceiveName::new_unchecked("BictoryNFT.burn");
        let burn = send(
            &sales.token.contract,
            receive_name,
            Amount::zero(),
            &sales.token.id,
        );
        actions = actions.and_then(burn);

        // Minting Token to the target contract
        receive_name = ReceiveName::new_unchecked("BictoryNFT.mint");
        let mint_data = MintData {
            token_id: sales.token.id,
            price: sales.price,
            for_sale: sales.for_sale,
            cid: sales.cid.clone(),
            hash: sales.hash.clone(),
            creator: Address::Account(sales.creator),
            creator_royalty: sales.creator_royalty,
            minter: Address::Account(sales.minter),
            minter_royalty: sales.minter_royalty,
        };
        let mut mint_data_set = Set::default();
        mint_data_set.insert(mint_data);
        let mint_data = MintingParameter {
            owner: Address::Account(params.owner),
            mint_data: mint_data_set,
        };
        let mint = send(
            &sales.token.contract,
            receive_name,
            Amount::zero(),
            &mint_data,
        );

        actions = actions.and_then(mint);

        // Event for puting NFT on sale / unsale.
        logger.log(&CustomEvent::SetForSale(SetForSaleEvent {
            owner: ctx.sender(),
            token_id: sales.token.id,
            for_sale: sales.for_sale,
        }))?;
    }
    Ok(actions)
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
        let state = result.expect_report("Contract initialization failed!");

        // Check the state
        claim_eq!(
            state.token_details.len(),
            0,
            "No tokens should be initialized."
        );
    }
}
