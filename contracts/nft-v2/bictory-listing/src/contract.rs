use super::*;

/// Initialize the listing contract with an empty list of listings.
#[init(contract = "BictoryListing", parameter = "ContractAddress")]
fn contract_init(ctx: &impl HasInitContext) -> InitResult<State> {
    let storage_address: ContractAddress = ctx.parameter_cursor().get()?;
    // Construct the initial contract state.
    let state = State::new(storage_address);
    Ok(state)
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
    parameter = "ContractTokenId"
)]
fn contract_list<A: HasActions>(
    ctx: &impl HasReceiveContext,
    state: &mut State,
) -> ContractResult<A> {
    // Ensure that no other call is in progress
    ensure!(
        state.function_states.is_none(),
        CustomContractError::RequestInProgress.into()
    );

    let storage = StorageContract::new(&state.storage_address);
    let mut actions = A::accept();

    let token_id: ContractTokenId = ctx.parameter_cursor().get()?;

    let keys = vec![
        OWNER.as_ref(),
        CREATOR.as_ref(),
        CREATOR_ROYALTY.as_ref(),
        MINTER.as_ref(),
        MINTER_ROYALTY.as_ref(),
        PRICE.as_ref(),
        FOR_SALE.as_ref(),
    ];

    let nft_details = NFTDetails {
        token_id: token_id.clone(),
        ..Default::default()
    };
    state.function_states = Some(FunctionStates::List(nft_details));

    // Setting Operator
    actions = actions.and_then(storage.send_find(
        &ctx.self_address(),
        "BictoryListing.updateOperator",
        &Bytes(token_id.0.clone()),
    ));

    // Getting required fields
    actions = actions.and_then(storage.send_get(
        &ctx.self_address(),
        "BictoryListing.functionContinue",
        <&ByteSlice>::from(token_id.0.as_slice()),
        &keys,
    ));

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
    parameter = "ContractTokenId"
)]
fn contract_unlist<A: HasActions>(
    ctx: &impl HasReceiveContext,
    state: &mut State,
) -> ContractResult<A> {
    let token_id: ContractTokenId = ctx.parameter_cursor().get()?;
    let keys = vec![OWNER.as_ref(), FOR_SALE.as_ref()];
    let storage = StorageContract::new(&state.storage_address);
    let mut actions = A::accept();

    state.function_states = Some(FunctionStates::UnList(token_id.clone()));

    // Remove Operator
    let update_operator: UpdateOperatorParams = UpdateOperatorParams(vec![UpdateOperator {
        update: OperatorUpdate::Remove,
        operator: Address::Contract(ctx.self_address()),
    }]);
    let receive_name = ReceiveName::new_unchecked("BictoryStorage.updateOperator");
    actions = actions.and_then(send(
        &state
            .leaf_contract_address
            .expect("Leaf contract address cannot be `None`."),
        receive_name,
        Amount::zero(),
        &update_operator,
    ));

    actions = actions.and_then(storage.send_get(
        &ctx.self_address(),
        "BictoryListing.functionContinue",
        <&ByteSlice>::from(token_id.0.as_slice()),
        &keys,
    ));

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
    parameter = "u32",
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
    let bictory_royalty: u32 = ctx.parameter_cursor().get()?;
    let mut actions = A::accept();
    let for_sale = false;
    let storage = StorageContract::new(&state.storage_address);

    if let Some(FunctionStates::List(nft_details)) = &state.function_states {
        // Ensuring price of NFT is lesser or equal to the amount passed
        ensure!(nft_details.price <= amount, Cis1Error::InsufficientFunds);

        // Ensuring anyone can buy except owner
        ensure!(sender.ne(&nft_details.owner), ContractError::Unauthorized);

        // Ensuring Token is listed for sale
        ensure!(
            nft_details.for_sale,
            CustomContractError::TokenNotListedForSale.into()
        );

        // setting `for_sale` as false and new owner.
        actions = actions.and_then(storage.send_set(
            <&ByteSlice>::from(nft_details.token_id.0.as_slice()),
            &[
                StorageEntryRef::new(FOR_SALE, &for_sale),
                StorageEntryRef::new(OWNER, &sender),
            ],
        ));

        // Calculating shares
        let mut shares = calc_shares(
            nft_details.price,
            nft_details.creator_royalty as u64,
            nft_details.minter_royalty as u64,
            bictory_royalty as u64,
        );
        shares.adjust_owner_share();

        // Balance Transfer
        let receive_name = ReceiveName::new_unchecked("BictoryConfig.sendCCD");

        // TODO: Remove bictory_royalty parameter from input and replace BictorCnfig
        // contract with leaf contract.
        // Hardcoding address of config smart contract for security reasons.
        // In a rare case of Bictory's wallet address change this contract address must
        // also be required to be updated.
        let bictory_config_contract_address = ContractAddress {
            index: 1902,
            subindex: 0,
        };
        // This action is required as user can put `0` royalty for but if this
        // contract is envoked then there will be additional check for correct
        // royalty amount transfer.
        let send_bictory_share = send(
            &bictory_config_contract_address,
            receive_name,
            shares.bictory,
            &bictory_royalty,
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

        // Remove Operator
        let update_operator: UpdateOperatorParams = UpdateOperatorParams(vec![UpdateOperator {
            update: OperatorUpdate::Remove,
            operator: Address::Contract(ctx.self_address()),
        }]);
        let receive_name = ReceiveName::new_unchecked("BictoryStorage.updateOperator");
        actions = actions.and_then(send(
            &state
                .leaf_contract_address
                .expect("Leaf contract address cannot be `None`."),
            receive_name,
            Amount::zero(),
            &update_operator,
        ));

        // Event for buying NFT.
        logger.log(&CustomEvent::Buy(BuyEvent {
            for_sale,
            token_id: nft_details.token_id.clone(),
            seller: nft_details.owner,
            buyer: sender,
            owner_share: shares.owner,
            creator_share: shares.creator,
        }))?;
    } else {
        return Err(CustomContractError::OperationDoesNotExist.into());
    }

    Ok(actions)
}

#[receive(
    contract = "BictoryListing",
    name = "functionContinue",
    parameter = "StorageGetResponse",
    enable_logger
)]
fn contract_continue<A: HasActions>(
    ctx: &impl HasReceiveContext,
    logger: &mut impl HasLogger,
    state: &mut State,
) -> ContractResult<A> {
    let sender = match ctx.sender() {
        Address::Contract(contract) => contract,
        Address::Account(_) => return Err(CustomContractError::ContractOnly.into()),
    };

    ensure_eq!(sender, state.storage_address, ContractError::Unauthorized);

    let params: StorageGetResponse = ctx.parameter_cursor().get()?;
    let storage = StorageContract::new(&state.storage_address);

    let function_state = state
        .function_states
        .take()
        .ok_or(CustomContractError::NoRequestInProgress)?;

    match function_state {
        FunctionStates::List(mut nft_details) => {
            nft_details.owner = params.get(OWNER)?;
            nft_details.creator = params.get(CREATOR)?;
            nft_details.creator_royalty = params.get(CREATOR_ROYALTY)?;
            nft_details.minter = params.get(MINTER)?;
            nft_details.minter_royalty = params.get(MINTER_ROYALTY)?;
            nft_details.price = params.get(PRICE)?;
            nft_details.for_sale = params.get(FOR_SALE).unwrap_or(false);

            // Ensuring only owner of NFT can list.
            ensure!(
                ctx.invoker().eq(&nft_details.owner),
                CustomContractError::OnlyOwner.into()
            );

            // Ensuring Token is not listed for sale
            ensure!(
                !nft_details.for_sale,
                CustomContractError::TokenAlreadyListedForSale.into()
            );

            // Updating local state
            let for_sale = true;
            nft_details.for_sale = for_sale;

            state.function_states = Some(FunctionStates::List(nft_details.clone()));

            // Event for listing NFT.
            logger.log(&CustomEvent::Listing(ListingEvent {
                nft_details: nft_details.clone(),
            }))?;

            let action = storage.send_set(
                <&ByteSlice>::from(nft_details.token_id.0.as_slice()),
                &[StorageEntryRef::new(FOR_SALE, &for_sale)],
            );

            Ok(action)
        }
        FunctionStates::UnList(token_id) => {
            let owner = params.get(OWNER)?;
            let for_sale: bool = params.get(FOR_SALE)?;

            // Ensuring only owner of NFT can unlist.
            ensure!(
                ctx.invoker().eq(&owner),
                CustomContractError::OnlyOwner.into()
            );

            // Ensuring Token is listed for sale
            ensure!(for_sale, CustomContractError::TokenNotListedForSale.into());

            let new_for_sale = false;

            let action = storage.send_set(
                <&ByteSlice>::from(token_id.0.as_slice()),
                &[StorageEntryRef::new(FOR_SALE, &new_for_sale)],
            );

            state.function_states = None;

            // Event for listing NFT.
            logger.log(&CustomEvent::Unlisting(UnlistingEvent {
                token_id,
                for_sale: new_for_sale,
            }))?;

            Ok(action)
        }
    }
}

#[receive(
    contract = "BictoryListing",
    name = "updateOperator",
    parameter = "StorageFindResponse"
)]
fn contract_update_operator<A: HasActions>(
    ctx: &impl HasReceiveContext,
    state: &mut State,
) -> ContractResult<A> {
    let sender = match ctx.sender() {
        Address::Contract(contract) => contract,
        Address::Account(_) => return Err(CustomContractError::ContractOnly.into()),
    };

    ensure_eq!(sender, state.storage_address, ContractError::Unauthorized);

    let params: StorageFindResponse = ctx.parameter_cursor().get()?;
    let mut actions = A::accept();
    if let Some(leaf_contract_address) = params.contract {
        state.leaf_contract_address = Some(leaf_contract_address);

        let update_operator: UpdateOperatorParams = UpdateOperatorParams(vec![UpdateOperator {
            update: OperatorUpdate::Add,
            operator: Address::Contract(ctx.self_address()),
        }]);
        let receive_name = ReceiveName::new_unchecked("BictoryStorage.updateOperator");
        actions = actions.and_then(send(
            &leaf_contract_address,
            receive_name,
            Amount::zero(),
            &update_operator,
        ));
    } else {
        return Err(CustomContractError::UnknownToken.into());
    }

    Ok(actions)
}
