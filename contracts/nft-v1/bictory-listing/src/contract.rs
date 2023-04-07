use super::*;

/// Initialize the listing contract with an empty list of listings.
#[init(contract = "BictoryListing")]
fn init<S: HasStateApi>(
    _ctx: &impl HasInitContext,
    state_builder: &mut StateBuilder<S>,
) -> InitResult<State<S>> {
    Ok(State::empty(state_builder))
}

/// List or update the price of a list of NFTs.
///
/// During this operation, the contract address of this contract will be
/// added as operator.
///  
/// Will reject if not send by the NFT owner or if it fails to parse the
/// parameter.
#[receive(
    contract = "BictoryListing",
    name = "list",
    parameter = "ListParams",
    mutable,
    enable_logger
)]
fn list<S: HasStateApi, V: Read>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S, ReturnValueType = V>,
    logger: &mut impl HasLogger,
) -> ContractResult<()> {
    let sender = ctx.sender();
    let owner = get_account_address(sender)?;
    let params: ListParams = ctx.parameter_cursor().get()?;

    // Ensuring token is not already listed for sale
    ensure!(
        host.state().listings.contains(&params.token),
        CustomContractError::TokenAlreadyListedForSale.into()
    );

    // Getting token info from NFT contract
    let parameter = ViewTokenParams {
        owner: sender,
        token_id: params.token.id.clone(),
    };
    let entrypoint_name = EntrypointName::new_unchecked("viewToken");
    let (_, value) = host.invoke_contract(
        &params.token.contract,
        &parameter,
        entrypoint_name,
        Amount::zero(),
    )?;

    if let Some(mut owned_data) = value {
        let nft_data = NFTData::deserial(&mut owned_data)?;

        // Adding this contract as operator to the receiving contract
        get_update_operator_action(
            host,
            ctx.self_address(),
            &params.token.contract,
            OperatorUpdate::Add,
        )?;

        // Update price
        get_update_price_action(host, &params.token.clone(), nft_data.price)?;

        host.state_mut().list(&params.token, owner, nft_data)?;

        // Event for listing NFT.
        logger.log(&CustomEvent::Listing(params))?;
    } else {
        return Err(ContractError::InvalidTokenId);
    }

    Ok(())
}

/// Remove NFTs from the listing.
///
/// Remember that operator will not be removed during this
/// operation as their might be another listing operations that
/// still need operator accounts. Token owner has to manually
/// remove the operator.
///
/// Rejects if
/// - Not send by the NFT owner.
/// - It fails to parse the parameter.
/// - Any of the tokens are not listed.
#[receive(
    contract = "BictoryListing",
    name = "unlist",
    parameter = "ListParams",
    mutable,
    enable_logger
)]
fn unlist<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
    logger: &mut impl HasLogger,
) -> ContractResult<()> {
    let sender = ctx.sender();
    let unlisting: ListParams = ctx.parameter_cursor().get()?;

    // Ensuring only owner of NFT can unlist the tokens
    ensure!(
        sender.matches_account(&unlisting.owner),
        CustomContractError::OnlyOwner.into()
    );

    host.state_mut().unlist(&unlisting.token)?;

    // Event for unlisting NFT.
    logger.log(&CustomEvent::Unlisting(unlisting))?;

    Ok(())
}

/// Buy one of the listed NFTs.
///
/// Rejects if:
/// - Sender is a contract address.
/// - It fails to parse the parameter.
/// - The token is not listed
/// - The amount is less then the listed price.
/// - The NFT contract transfer rejects.
#[receive(
    contract = "BictoryListing",
    name = "buy",
    parameter = "BuyParams",
    mutable,
    enable_logger,
    payable
)]
fn buy<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
    price: Amount,
    logger: &mut impl HasLogger,
) -> ContractResult<()> {
    let sender = get_account_address(ctx.sender())?;
    let params: BuyParams = ctx.parameter_cursor().get()?;
    let nft_details = host.state_mut().unlist(&params.token)?;

    // Ensuring price of NFT is lesser or equal to the amount passed
    ensure!(nft_details.price <= price, ContractError::InsufficientFunds);

    // Ensuring anyone can buy except owner
    ensure!(sender.ne(&nft_details.owner), ContractError::Unauthorized);

    // Transfer action
    let transfer = Transfer {
        token_id: params.token.id.clone(),
        amount: ContractTokenAmount::from(1),
        from: Address::Account(nft_details.owner),
        to: Receiver::Account(sender),
        data: AdditionalData::empty(),
    };
    let parameter = TransferParams(vec![transfer]);
    let mut entrypoint_name = EntrypointName::new_unchecked("transfer");
    host.invoke_contract(
        &params.token.contract,
        &parameter,
        entrypoint_name,
        Amount::zero(),
    )?;

    // Calculating shares
    let mut shares = calc_shares(
        nft_details.price,
        nft_details.creator_royalty as u64,
        nft_details.minter_royalty as u64,
        params.bictory_royalty as u64,
    );
    shares.adjust_owner_share();

    // Balance Transfer
    entrypoint_name = EntrypointName::new_unchecked("sendCCD");

    // Hardcoding address of config smart contract for security reasons.
    // In a rare case of Bictory's wallet address change this contract address must
    // also be required to be updated.
    let bictory_config_contract_address = ContractAddress {
        index: 571,
        subindex: 0,
    };
    host.invoke_contract(
        &bictory_config_contract_address,
        &"",
        entrypoint_name,
        shares.bictory,
    )?;

    host.invoke_transfer(&nft_details.owner, shares.owner)?;

    // Creator Royalty can be `0` thereby avoiding unnecessary gas fees.
    if nft_details.creator_royalty != 0 {
        host.invoke_transfer(&nft_details.creator, shares.creator)?;
    }
    // Minter Royalty can be `0` thereby avoiding unnecessary gas fees.
    if nft_details.minter_royalty != 0 {
        host.invoke_transfer(&nft_details.minter, shares.minter)?;
    }

    // Event for buying NFT.
    logger.log(&CustomEvent::Buy(BuyEvent {
        token: params.token,
        seller: nft_details.owner,
        buyer: sender,
        owner_share: shares.owner,
        creator_share: shares.creator,
    }))?;

    Ok(())
}

/// Update the price of the listed NFT.
///  
/// Rejects if:
/// - Sender is not NFT owner.
/// - It fails to parse the parameter.
/// - The token is not listed.
#[receive(
    contract = "BictoryListing",
    name = "updatePrice",
    parameter = "UpdateListingPrice",
    mutable
)]
fn update_price<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<()> {
    let sender = ctx.sender();
    let params: UpdateListingPrice = ctx.parameter_cursor().get()?;

    get_update_price_action(host, &params.token, params.price)?;

    if let Some(mut listing) = host.state().listings.get_mut(&params.token) {
        // Ensuring only owner of NFT can list and alter with parameters
        // during listing.
        ensure!(
            sender.matches_account(&listing.owner),
            CustomContractError::OnlyOwner.into()
        );

        // Update price
        listing.price = params.price;
    } else {
        return Err(CustomContractError::TokenNotListedForSale.into());
    }

    Ok(())
}

/// View function that returns the contents of the NFTDetails
/// that is listed by given token_id
#[receive(
    contract = "BictoryListing",
    name = "view",
    parameter = "Token",
    return_value = "NFTDetails"
)]
fn view<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &impl HasHost<State<S>, StateApiType = S>,
) -> ReceiveResult<NFTDetails> {
    let token: Token = ctx.parameter_cursor().get()?;
    let state = host.state();

    Ok(*state
        .listings
        .get(&token)
        .ok_or(CustomContractError::UnknownToken)?)
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
        let mut builder = TestStateBuilder::new();

        // Call the contract function.
        let result = init(&ctx, &mut builder);

        // Check the result
        let state = result.expect_report("Contract initialization failed");

        // Check the state
        claim_eq!(
            state.listings.iter().count(),
            0,
            "No listings should be initialized"
        );
    }
}
