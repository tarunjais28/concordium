use super::*;

/// Initialize the listing contract with an empty list of listings.
#[init(contract = "BictoryListing")]
fn contract_init(_ctx: &impl HasInitContext) -> InitResult<State> {
    Ok(State::empty())
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
    enable_logger
)]
fn contract_list<A: HasActions>(
    ctx: &impl HasReceiveContext,
    logger: &mut impl HasLogger,
    state: &mut State,
) -> ContractResult<A> {
    let sender = ctx.sender();
    let listing: ListParams = ctx.parameter_cursor().get()?;

    let mut actions = A::accept();
    let for_sale = true;
    // Ensuring only owner of NFT can list and alter with parameters
    // during listing.
    ensure!(
        sender.matches_account(&listing.owner),
        CustomContractError::OnlyOwner.into()
    );

    // Ensuring Token is not listed for sale
    ensure!(
        !listing.for_sale,
        CustomContractError::TokenAlreadyListedForSale.into()
    );

    // Add operator only if state is empty
    if state.listings.keys().len() == 0 {
        // Adding this contract as operator to the receiving contract
        actions = actions.and_then(get_update_operator_action(
            ctx.self_address(),
            &listing.token.contract,
            OperatorUpdate::Add,
        ));
    }

    // Update price
    actions = actions.and_then(get_update_price_action(&listing.token, listing.price));

    state.list(listing.clone(), for_sale);

    // setting `for_sale` as true.
    let data = AdditionalData::from(vec![1]);
    // Transfer action to set `for_sale` flag.
    let transfer = Transfer {
        token_id: listing.token.id.clone(),
        amount: 1,
        from: Address::Account(listing.owner),
        to: Receiver::Account(listing.owner),
        data,
    };
    let parameter = TransferParams(vec![transfer]);
    let receive_name = ReceiveName::new_unchecked("BictoryNFT.transfer");
    actions = actions.and_then(send(
        &listing.token.contract,
        receive_name,
        Amount::zero(),
        &parameter,
    ));

    // Event for listing NFT.
    logger.log(&CustomEvent::Listing(ListingEvent { for_sale, listing }))?;

    Ok(actions)
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
/// - it fails to parse the parameter.
/// - Any of the tokens are not listed.
#[receive(
    contract = "BictoryListing",
    name = "unlist",
    parameter = "UnlistParams",
    enable_logger
)]
fn contract_unlist<A: HasActions>(
    ctx: &impl HasReceiveContext,
    logger: &mut impl HasLogger,
    state: &mut State,
) -> ContractResult<A> {
    let sender = ctx.sender();
    let unlisting: UnlistParams = ctx.parameter_cursor().get()?;

    let mut actions = A::accept();
    let for_sale = false;
    // Ensuring only owner of NFT can unlist the tokens
    ensure!(
        sender.matches_account(&unlisting.owner),
        CustomContractError::OnlyOwner.into()
    );

    state.unlist(&unlisting.token)?;

    // setting `for_sale` as false.
    let data = AdditionalData::from(vec![0]);
    // Transfer action to unset `for_sale` flag
    let transfer = Transfer {
        token_id: unlisting.token.id.clone(),
        amount: 1,
        from: Address::Account(unlisting.owner),
        to: Receiver::Account(unlisting.owner),
        data,
    };
    let parameter = TransferParams(vec![transfer]);
    let receive_name = ReceiveName::new_unchecked("BictoryNFT.transfer");
    actions = actions.and_then(send(
        &unlisting.token.contract,
        receive_name,
        Amount::zero(),
        &parameter,
    ));

    // Remove operator only if state is empty
    if state.listings.keys().len() == 0 {
        // Removing this contract as operator to the receiving contract
        actions = actions.and_then(get_update_operator_action(
            ctx.self_address(),
            &unlisting.token.contract,
            OperatorUpdate::Remove,
        ));
    }

    // Event for unlisting NFT.
    logger.log(&CustomEvent::Unlisting(UnlistingEvent {
        for_sale,
        token: unlisting.token,
    }))?;

    Ok(actions)
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
    enable_logger,
    payable
)]
fn contract_buy<A: HasActions>(
    ctx: &impl HasReceiveContext,
    amount: Amount,
    logger: &mut impl HasLogger,
    state: &mut State,
) -> ContractResult<A> {
    let sender = match ctx.sender() {
        Address::Account(addr) => addr,
        Address::Contract(_) => bail!(CustomContractError::OnlyAccountAddress.into()),
    };
    let params: BuyParams = ctx.parameter_cursor().get()?;
    let nft_details = state.unlist(&params.token)?;
    let for_sale = false;
    let mut actions = A::accept();

    // Ensuring price of NFT is lesser or equal to the amount passed
    ensure!(
        nft_details.price <= amount,
        CustomContractError::InsufficientAmount.into()
    );

    // Ensuring anyone can buy except owner
    ensure!(sender.ne(&nft_details.owner), ContractError::Unauthorized);

    // Ensuring Token is listed for sale
    ensure!(
        nft_details.for_sale,
        CustomContractError::TokenNotListedForSale.into()
    );

    // setting `for_sale` as false.
    let data = AdditionalData::from(vec![0]);
    // Transfer action
    let transfer = Transfer {
        token_id: params.token.id.clone(),
        amount: 1,
        from: Address::Account(nft_details.owner),
        to: Receiver::Account(sender),
        data,
    };
    let parameter = TransferParams(vec![transfer]);
    let mut receive_name = ReceiveName::new_unchecked("BictoryNFT.transfer");
    actions = actions.and_then(send(
        &params.token.contract,
        receive_name,
        Amount::zero(),
        &parameter,
    ));

    // Calculating shares
    let mut shares = calc_shares(
        nft_details.price,
        nft_details.creator_royalty as u64,
        nft_details.minter_royalty as u64,
        params.bictory_royalty as u64,
    );
    shares.adjust_owner_share();

    // Balance Transfer
    receive_name = ReceiveName::new_unchecked("BictoryConfig.sendCCD");

    // Hardcoding address of config smart contract for security reasons.
    // In a rare case of Bictory's wallet address change this contract address must
    // also be required to be updated.
    let bictory_config_contract_address = ContractAddress {
        index: 71,
        subindex: 0,
    };
    // This action is required as user can put `0` royalty for but if this
    // contract is envoked then there will be additional check for correct
    // royalty amount transfer.
    let send_bictory_share = send(
        &bictory_config_contract_address,
        receive_name,
        shares.bictory,
        &"",
    );
    actions = actions.and_then(send_bictory_share);
    actions = actions.and_then(A::simple_transfer(&nft_details.owner, shares.owner));

    // Creator Royalty can be `0` thereby avoiding unnecessary gas fees.
    if nft_details.creator_royalty != 0 {
        actions = actions.and_then(A::simple_transfer(&nft_details.creator, shares.creator));
    }
    // Minter Royalty can be `0` thereby avoiding unnecessary gas fees.
    if nft_details.minter_royalty != 0 {
        actions = actions.and_then(A::simple_transfer(&nft_details.minter, shares.minter));
    }

    // Event for buying NFT.
    logger.log(&CustomEvent::Buy(BuyEvent {
        for_sale,
        token: params.token,
        seller: nft_details.owner,
        buyer: sender,
        owner_share: shares.owner,
        creator_share: shares.creator,
    }))?;

    Ok(actions)
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
    parameter = "UpdateListingPrice"
)]
fn contract_update_price<A: HasActions>(
    ctx: &impl HasReceiveContext,
    state: &mut State,
) -> ContractResult<A> {
    let sender = ctx.sender();
    let params: UpdateListingPrice = ctx.parameter_cursor().get()?;

    let mut actions = A::accept();

    if let Some(listing) = state.listings.get_mut(&params.token) {
        // Ensuring only owner of NFT can list and alter with parameters
        // during listing.
        ensure!(
            sender.matches_account(&listing.owner),
            CustomContractError::OnlyOwner.into()
        );

        // Update price
        listing.price = params.price;
        actions = actions.and_then(get_update_price_action(&params.token, listing.price));
    } else {
        return Err(CustomContractError::TokenNotListedForSale.into());
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
        let state = result.expect_report("Contract initialization failed");

        // Check the state
        claim_eq!(state.listings.len(), 0, "No listings should be initialized");
    }
}
