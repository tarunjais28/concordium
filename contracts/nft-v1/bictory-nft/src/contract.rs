use super::*;

/// Initialize contract instance with no token types initially.
#[init(contract = "BictoryNFT")]
fn init<S: HasStateApi>(
    _ctx: &impl HasInitContext,
    state_builder: &mut StateBuilder<S>,
) -> InitResult<State<S>> {
    // Construct the initial contract state.
    let state = State::empty(state_builder);
    Ok(state)
}

/// Mint new tokens with a given address as the owner of these tokens.
/// Logs a `Mint` and a `TokenMetadata` event for each token.
/// The url for the token metadata is the token ID encoded in hex, appended on
/// the `TOKEN_METADATA_BASE_URL`.
/// Ow&mut &mut ner and Minter will be same during minting.
///
/// It rejects if:
/// - The sender is not the contract instance owner.
/// - Fails to parse parameter.
/// - Any of the tokens fails to be minted, which could be if:
///     - The minted token ID already exists.
///     - Fails to log Mint event
///     - Fails to log TokenMetadata event
///
/// Note: Can at most mint 32 token types in one call due to the limit on the
/// number of logs a smart contract can produce on each function call.
#[receive(
    contract = "BictoryNFT",
    name = "mint",
    parameter = "MintParams",
    mutable,
    enable_logger,
    payable
)]
fn mint<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
    price: Amount,
    logger: &mut impl HasLogger,
) -> ContractResult<()> {
    // Parse the parameter.
    let mint_data: MintParams = ctx.parameter_cursor().get()?;

    let (state, state_builder) = host.state_and_builder();
    let token_id = mint_data.token_id.clone();

    // Mint the token in the state.
    state.mint(mint_data.clone(), price, state_builder)?;

    if mint_data.owner.ne(&mint_data.creator) {
        // Royalty is set to 20% when creator and minter are different
        let bictory_royalty = 20_000_000;
        let royalty_to_creator: u64 = 100_000_000 - bictory_royalty;
        let shares = calc_shares(price, royalty_to_creator, 0, bictory_royalty);

        if let Address::Account(creator) = mint_data.creator {
            host.invoke_transfer(&creator, shares.creator)?;
        }

        let entrypoint_name = EntrypointName::new_unchecked("sendCCD");

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
    }

    // Event for minted NFT.
    logger.log(&Cis2Event::Mint(MintEvent {
        token_id: token_id.clone(),
        amount: mint_data.quantity,
        owner: mint_data.owner,
    }))?;

    // Metadata URL for the NFT.
    logger.log(&token_metadata_event(token_id))?;

    Ok(())
}

/// Execute a list of token transfers, in the order of the list.
///
/// Logs a `Transfer` event for each transfer in the list.
/// Produces an action which sends a message to each contract which are the
/// receiver of a transfer.
///
/// It rejects if:
/// - It fails to parse the parameter.
/// - Any of the transfers fail to be executed, which could be if:
///     - The `token_id` does not exist.
///     - The sender is not the owner of the token, or an operator for this
///       specific `token_id` and `from` address.
///     - The token is not owned by the `from`.
/// - Fails to log event.
/// - Any of the messages sent to contracts receiving a transfer choose to
///   reject.
#[receive(
    contract = "BictoryNFT",
    name = "transfer",
    parameter = "TransferParameter",
    mutable,
    enable_logger
)]
fn transfer<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
    logger: &mut impl HasLogger,
) -> ContractResult<()> {
    // Parse the parameter.
    let TransferParams(transfers): TransferParameter = ctx.parameter_cursor().get()?;
    // Get the sender who invoked this contract function.
    let sender = ctx.sender();

    for transfer in transfers {
        let (state, state_builder) = host.state_and_builder();
        // Authenticate the sender for this transfer
        ensure!(
            transfer.from == sender || state.is_operator(&sender, &transfer.from),
            ContractError::Unauthorized
        );

        let to_address = transfer.to.address();
        // Update the contract state
        state.transfer(&transfer, state_builder)?;

        // Log transfer event
        logger.log(&Cis2Event::Transfer(TransferEvent {
            token_id: transfer.token_id.clone(),
            amount: transfer.amount,
            from: transfer.from,
            to: to_address,
        }))?;

        // If the receiver is a contract, we add sending it a message to the list of
        // actions.
        if let Receiver::Contract(address, entrypoint_name) = transfer.to {
            let parameter = OnReceivingCis2Params {
                token_id: transfer.token_id,
                amount: transfer.amount,
                from: transfer.from,
                data: transfer.data,
            };

            host.invoke_contract(
                &address,
                &parameter,
                entrypoint_name.as_entrypoint_name(),
                Amount::zero(),
            )?;
        }
    }
    Ok(())
}

/// Enable or disable addresses as operators of the sender address.
/// Logs an `UpdateOperator` event.
///
/// It rejects if:
/// - It fails to parse the parameter.
/// - The operator address is the same as the sender address.
/// - Fails to log event.
#[receive(
    contract = "BictoryNFT",
    name = "updateOperator",
    parameter = "UpdateOperatorParams",
    mutable,
    enable_logger
)]
fn update_operator<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
    logger: &mut impl HasLogger,
) -> ContractResult<()> {
    // Parse the parameter.
    let UpdateOperatorParams(params) = ctx.parameter_cursor().get()?;
    // Get the sender who invoked this contract function.
    let sender = Address::Account(ctx.invoker());

    let (state, state_builder) = host.state_and_builder();
    for param in params {
        // Update the operator in the state.
        match param.update {
            OperatorUpdate::Add => state.add_operator(&sender, &param.operator, state_builder),
            OperatorUpdate::Remove => state.remove_operator(&sender, &param.operator),
        }

        // Log the appropriate event
        logger.log(
            &Cis2Event::<ContractTokenId, ContractTokenAmount>::UpdateOperator(
                UpdateOperatorEvent {
                    owner: sender,
                    operator: param.operator,
                    update: param.update,
                },
            ),
        )?;
    }

    Ok(())
}

/// Takes a list of queries. Each query is an owner address and some address to
/// check as an operator of the owner address. It takes a contract address plus
/// contract function to invoke with the result.
///
/// It rejects if:
/// - It fails to parse the parameter.
/// - Message sent back with the result rejects.
#[receive(
    contract = "BictoryNFT",
    name = "operatorOf",
    parameter = "OperatorOfQueryParams",
    return_value = "OperatorOfQueryResponse"
)]
fn operator_of<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &impl HasHost<State<S>>,
) -> ContractResult<OperatorOfQueryResponse> {
    // Parse the parameter.
    let params: OperatorOfQueryParams = ctx.parameter_cursor().get()?;
    // Build the response.
    let mut response = Vec::with_capacity(params.queries.len());
    let state = host.state();
    for query in params.queries {
        // Query the state for address being an operator of owner.
        let is_operator = state.is_operator(&query.owner, &query.address);
        response.push(is_operator);
    }

    Ok(OperatorOfQueryResponse::from(response))
}

/// Get the balance of given token IDs and addresses. It takes a contract
/// address plus contract function to invoke with the result.
///
/// It rejects if:
/// - It fails to parse the parameter.
/// - Any of the queried `token_id` does not exist.
/// - Message sent back with the result rejects.
#[receive(
    contract = "BictoryNFT",
    name = "balanceOf",
    parameter = "ContractBalanceOfQueryParams",
    return_value = "ContractBalanceOfQueryResponse"
)]
fn balance_of<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &impl HasHost<State<S>>,
) -> ContractResult<ContractBalanceOfQueryResponse> {
    // Parse the parameter.
    let params: ContractBalanceOfQueryParams = ctx.parameter_cursor().get()?;
    // Build the response.
    let mut response = Vec::with_capacity(params.queries.len());
    let state = host.state();
    for query in params.queries {
        // Query the state for balance.
        let amount = state.balance(&query.token_id, &query.address)?;
        response.push(amount);
    }

    Ok(ContractBalanceOfQueryResponse::from(response))
}

/// NFT Burn Functionality.
/// Can only be called by token owner.
/// Logs a `Burn` and a `TokenMetadata` event for each token.
/// The url for the token metadata is the token ID encoded in hex, appended on
/// the `TOKEN_METADATA_BASE_URL`.
///
/// It rejects if:
/// - The sender is not the token owner.
/// - Fails to parse parameter.
/// - Tokens fails to be upated, which could be if:
///     - The minted token ID does not exist.
///     - Fails to log Burn event
///     - Fails to log TokenMetadata event
#[receive(
    contract = "BictoryNFT",
    name = "burn",
    parameter = "BurnParams",
    mutable,
    enable_logger
)]
fn burn<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>>,
    logger: &mut impl HasLogger,
) -> ContractResult<()> {
    // Parse the parameter.
    let params: BurnParams = ctx.parameter_cursor().get()?;

    let sender = ctx.sender();
    let state = host.state_mut();

    // Authenticate the sender for this burn
    ensure!(
        params.owner == sender || state.is_operator(&sender, &params.owner),
        ContractError::Unauthorized
    );

    // Burning NFT
    let event = state.burn(&params.owner, params.clone())?;

    // Event for burning NFT.
    logger.log(&Cis2Event::Burn(event))?;

    // Metadata URL for the NFT.
    logger.log(&token_metadata_event(params.token_id))?;

    Ok(())
}

/// NFT Update Price Functionality.
/// Can only be called by token owner.
/// Logs a `UpdatePrice` and a `TokenMetadata` event for each token.
/// The url for the token metadata is the token ID encoded in hex, appended on
/// the `TOKEN_METADATA_BASE_URL`.
///
/// It rejects if:
/// - The sender is not the token owner.
/// - Fails to parse parameter.
/// - Tokens fails to be upated, which could be if:
///     - The minted token ID does not exist.
///     - Fails to log UpdatePrice event
///     - Fails to log TokenMetadata event
#[receive(
    contract = "BictoryNFT",
    name = "updatePrice",
    parameter = "UpdatePriceParameter",
    mutable,
    enable_logger
)]
fn update_price<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>>,
    logger: &mut impl HasLogger,
) -> ContractResult<()> {
    // Parse the parameter.
    let params: UpdatePriceParameter = ctx.parameter_cursor().get()?;

    let token_id = params.token_id.clone();
    let state = host.state_mut();

    // Updating Price
    let event = state.update_price(&Address::Account(ctx.invoker()), params)?;

    // Event for updating price of NFT.
    logger.log(&CustomEvent::UpdatePrice(event))?;

    // Metadata URL for the NFT.
    logger.log(&token_metadata_event(token_id))?;

    Ok(())
}

fn token_metadata_event(
    token_id: ContractTokenId,
) -> Cis2Event<ContractTokenId, ContractTokenAmount> {
    let token_metadata_url = build_token_metadata_url(&token_id);
    Cis2Event::TokenMetadata(TokenMetadataEvent {
        token_id,
        metadata_url: MetadataUrl {
            url: token_metadata_url,
            hash: None,
        },
    })
}

/// View tokens owned by particular address.
#[receive(
    contract = "BictoryNFT",
    name = "viewAddressStateByOwner",
    parameter = "Address",
    return_value = "ViewAddressState"
)]
fn view_address_state_by_owner<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &impl HasHost<State<S>, StateApiType = S>,
) -> ReceiveResult<ViewAddressState> {
    // Parse the parameter.
    let owner: Address = ctx.parameter_cursor().get()?;
    let mut view_addr_state = ViewAddressState::new();

    if let Some(addr_state) = host.state().state.get(&owner) {
        let operators = addr_state.operators.iter().map(|x| *x).collect();
        view_addr_state.operators = operators;

        for (token_id, owned_data) in addr_state.owned_tokens.iter() {
            view_addr_state
                .owned_data
                .insert(token_id.clone(), owned_data.as_nft_data());
        }
    };

    Ok(view_addr_state)
}

/// View token data owned by particular address by token_id.
#[receive(
    contract = "BictoryNFT",
    name = "viewToken",
    parameter = "ViewTokenParams",
    return_value = "NFTData"
)]
fn view_token<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &impl HasHost<State<S>, StateApiType = S>,
) -> ReceiveResult<NFTData> {
    // Parse the parameter.
    let params: ViewTokenParams = ctx.parameter_cursor().get()?;

    let addr_state = host
        .state()
        .state
        .get(&params.owner)
        .ok_or(ContractError::Custom(CustomContractError::AddressNotFound))?;
    let owned_data = addr_state
        .owned_tokens
        .get(&params.token_id)
        .ok_or(ContractError::InvalidTokenId)?;

    Ok(owned_data.as_nft_data())
}

#[concordium_cfg_test]
mod tests {
    use super::*;
    use test_infrastructure::*;

    const ACCOUNT_0: AccountAddress = AccountAddress([0u8; 32]);
    const ADDRESS_0: Address = Address::Account(ACCOUNT_0);
    const ACCOUNT_1: AccountAddress = AccountAddress([1u8; 32]);
    const ADDRESS_1: Address = Address::Account(ACCOUNT_1);

    fn token_0() -> ContractTokenId {
        TokenIdVec(vec![0, 1])
    }

    fn token_1() -> ContractTokenId {
        TokenIdVec(vec![42, 84, 168])
    }

    fn get_mint_data(
        owner: Address,
        creator: Address,
        token_id: ContractTokenId,
        quantity: ContractTokenAmount,
    ) -> MintData<ContractTokenId> {
        MintData {
            token_id,
            cid: Vec::new(),
            creator_royalty: 0,
            minter_royalty: 0,
            creator,
            quantity,
            owner,
        }
    }

    fn new_mint_params(
        owner: Address,
        creator: Address,
        token_id: ContractTokenId,
        quantity: ContractTokenAmount,
    ) -> MintParams {
        get_mint_data(owner, creator, token_id, quantity)
    }

    /// Test helper function which creates a contract state with two tokens with
    /// id `token_0` and id `token_1` owned by `ADDRESS_0`
    fn initial_state<S: HasStateApi>(
        state_builder: &mut StateBuilder<S>,
        quantity: ContractTokenAmount,
    ) -> State<S> {
        let mut state = State::empty(state_builder);
        let price = Amount::zero();

        // parameter
        let mut mint_data = get_mint_data(ADDRESS_0, ADDRESS_0, token_0(), quantity);

        state
            .mint(mint_data, price, state_builder)
            .expect_report("Failed to mint token_0");

        // parameter
        mint_data = get_mint_data(ADDRESS_1, ADDRESS_1, token_1(), quantity);

        state
            .mint(mint_data, price, state_builder)
            .expect_report("Failed to mint token_1");

        state
    }

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
            state.all_tokens.iter().count(),
            0,
            "No token should be initialized"
        );
    }

    /// Test minting, ensuring the new tokens are owned by the given address and
    /// the appropriate events are logged.
    #[concordium_test]
    fn test_mint() {
        let quantity_1 = ContractTokenAmount::from(1);

        // Setup the context
        let mut ctx = TestReceiveContext::empty();
        ctx.set_sender(ADDRESS_0);
        ctx.set_owner(ACCOUNT_0);
        ctx.set_invoker(ACCOUNT_0);

        let mint_data = new_mint_params(ADDRESS_0, ADDRESS_0, token_0(), quantity_1);

        let parameter_bytes = to_bytes(&mint_data);
        ctx.set_parameter(&parameter_bytes);

        let mut logger = TestLogger::init();
        let mut state_builder = TestStateBuilder::new();
        let state = State::empty(&mut state_builder);
        let mut host = TestHost::new(state, state_builder);

        // Call the contract function.
        let result: ContractResult<()> = mint(&ctx, &mut host, Amount::zero(), &mut logger);

        // Check the result
        claim!(result.is_ok(), "Results in rejection");

        // Check the state
        claim_eq!(
            host.state().all_tokens.iter().count(),
            1,
            "Expected three tokens in the state."
        );

        let balance0 = host
            .state()
            .balance(&token_0(), &ADDRESS_0)
            .expect_report("Token is expected to exist");
        claim_eq!(
            balance0,
            quantity_1,
            "Tokens should be owned by the given address 0"
        );

        // Check the logs
        claim!(
            logger.logs.contains(&to_bytes(&Cis2Event::Mint(MintEvent {
                owner: ADDRESS_0,
                token_id: token_0(),
                amount: quantity_1,
            }))),
            "Expected an event for minting token_0"
        );
    }

    /// Test transfer succeeds, when `from` is the sender.
    #[concordium_test]
    fn test_transfer_account() {
        let quantity_1 = ContractTokenAmount::from(1);

        // Setup the context
        let mut ctx = TestReceiveContext::empty();
        ctx.set_sender(ADDRESS_0);

        // and parameter.
        let transfer = Transfer {
            token_id: token_0(),
            from: ADDRESS_0,
            to: Receiver::from_account(ACCOUNT_1),
            amount: quantity_1,
            data: AdditionalData::empty(),
        };
        let parameter = TransferParams(vec![transfer]);
        let parameter_bytes = to_bytes(&parameter);
        ctx.set_parameter(&parameter_bytes);

        let mut logger = TestLogger::init();
        let mut state_builder = TestStateBuilder::new();
        let state = initial_state(&mut state_builder, quantity_1);
        let mut host = TestHost::new(state, state_builder);

        // Call the contract function.
        let result: ContractResult<()> = self::transfer(&ctx, &mut host, &mut logger);
        // Check the result.
        claim!(result.is_ok(), "Results in rejection");

        // Check the logs.
        claim_eq!(logger.logs.len(), 1, "Only one event should be logged");
        claim_eq!(
            logger.logs[0],
            to_bytes(&Cis2Event::Transfer(TransferEvent {
                from: ADDRESS_0,
                to: ADDRESS_1,
                token_id: token_0(),
                amount: quantity_1,
            })),
            "Incorrect event emitted"
        )
    }

    /// Test transfer token fails, when sender is neither the owner or an
    /// operator of the owner.
    #[concordium_test]
    fn test_transfer_not_authorized() {
        let quantity_1 = ContractTokenAmount::from(1);
        
        // Setup the context
        let mut ctx = TestReceiveContext::empty();
        ctx.set_sender(ADDRESS_1);

        // and parameter.
        let transfer = Transfer {
            token_id: token_0(),
            from: ADDRESS_0,
            amount: quantity_1,
            to: Receiver::from_account(ACCOUNT_1),
            data: AdditionalData::empty(),
        };
        let parameter = TransferParams(vec![transfer]);
        let parameter_bytes = to_bytes(&parameter);
        ctx.set_parameter(&parameter_bytes);

        let mut logger = TestLogger::init();
        let mut state_builder = TestStateBuilder::new();
        let state = State::empty(&mut state_builder);
        let mut host = TestHost::new(state, state_builder);

        // Call the contract function.
        let result: ContractResult<()> = self::transfer(&ctx, &mut host, &mut logger);
        // Check the result.
        let err = result.expect_err_report("Expected to fail");
        claim_eq!(
            err,
            ContractError::Unauthorized,
            "Error is expected to be Unauthorized"
        )
    }

    /// Test transfer succeeds when sender is not the owner, but is an operator
    /// of the owner.
    #[concordium_test]
    fn test_operator_transfer() {
        let quantity_1 = ContractTokenAmount::from(1);
        let quantity_2 = ContractTokenAmount::from(2);
        let quantity_3 = ContractTokenAmount::from(3);

        // Setup the context
        let mut ctx = TestReceiveContext::empty();
        ctx.set_sender(ADDRESS_1);

        // and parameter.
        let transfer = Transfer {
            from: ADDRESS_0,
            to: Receiver::from_account(ACCOUNT_1),
            token_id: token_0(),
            amount: quantity_2,
            data: AdditionalData::empty(),
        };
        let parameter = TransferParams::from(vec![transfer]);
        let parameter_bytes = to_bytes(&parameter);
        ctx.set_parameter(&parameter_bytes);

        let mut logger = TestLogger::init();
        let mut state_builder = TestStateBuilder::new();
        let mut state = initial_state(&mut state_builder, quantity_3);
        state.add_operator(&ADDRESS_0, &ADDRESS_1, &mut state_builder);
        let mut host = TestHost::new(state, state_builder);

        // Call the contract function.
        let result: ContractResult<()> = self::transfer(&ctx, &mut host, &mut logger);

        // Check the result.
        claim!(result.is_ok(), "Results in rejection");

        // Check the state.
        let balance0 = host
            .state()
            .balance(&token_0(), &ADDRESS_0)
            .expect_report("Token is expected to exist");
        let balance1 = host
            .state()
            .balance(&token_0(), &ADDRESS_1)
            .expect_report("Token is expected to exist");
        claim_eq!(
            balance0,
            quantity_1,
            "Token owner balance should be decreased by the transferred amount after first transfer."
        );
        claim_eq!(
            balance1,
            quantity_2,
            "Token receiver balance should be increased by the transferred amount after first transfer."
        );

        // Check the logs.
        claim_eq!(logger.logs.len(), 1, "Only one event should be logged");
        claim_eq!(
            logger.logs[0],
            to_bytes(&Cis2Event::Transfer(TransferEvent {
                from: ADDRESS_0,
                to: ADDRESS_1,
                token_id: token_0(),
                amount: quantity_2,
            })),
            "Incorrect event emitted"
        );

        // and parameter.
        let transfer = Transfer {
            from: ADDRESS_0,
            to: Receiver::from_account(ACCOUNT_1),
            token_id: token_0(),
            amount: quantity_1,
            data: AdditionalData::empty(),
        };
        let parameter = TransferParams::from(vec![transfer]);
        let parameter_bytes = to_bytes(&parameter);
        ctx.set_parameter(&parameter_bytes);

        // Call the contract function.
        let _: ContractResult<()> = self::transfer(&ctx, &mut host, &mut logger);

        // Check the state.
        let balance0 = host
            .state()
            .balance(&token_0(), &ADDRESS_0)
            .expect_report("Token is expected to exist");
        let balance1 = host
            .state()
            .balance(&token_0(), &ADDRESS_1)
            .expect_report("Token is expected to exist");
        claim_eq!(
            balance0,
            ContractTokenAmount::from(0),
            "Token owner balance should be decreased by the transferred amount after second transfer."
        );
        claim_eq!(
            balance1,
            quantity_3,
            "Token receiver balance should be increased by the transferred amount after second transfer."
        );
    }

    /// Test adding an operator succeeds and the appropriate event is logged.
    #[concordium_test]
    fn test_add_operator() {
        // Setup the context
        let mut ctx = TestReceiveContext::empty();
        ctx.set_sender(ADDRESS_0);
        ctx.set_invoker(ACCOUNT_0);
        ctx.set_invoker(ACCOUNT_0);

        // and parameter.
        let update = UpdateOperator {
            update: OperatorUpdate::Add,
            operator: ADDRESS_1,
        };
        let parameter = UpdateOperatorParams(vec![update]);
        let parameter_bytes = to_bytes(&parameter);
        ctx.set_parameter(&parameter_bytes);

        let mut logger = TestLogger::init();
        let mut state_builder = TestStateBuilder::new();
        let state = State::empty(&mut state_builder);
        let mut host = TestHost::new(state, state_builder);

        // Call the contract function.
        let result: ContractResult<()> = update_operator(&ctx, &mut host, &mut logger);

        // Check the result.
        claim!(result.is_ok(), "Results in rejection");

        // Check the state.
        let is_operator = host.state().is_operator(&ADDRESS_1, &ADDRESS_0);
        claim!(is_operator, "Account should be an operator");

        // Check the logs.
        claim_eq!(logger.logs.len(), 1, "One event should be logged");
        claim_eq!(
            logger.logs[0],
            to_bytes(
                &Cis2Event::<ContractTokenId, ContractTokenAmount>::UpdateOperator(
                    UpdateOperatorEvent {
                        owner: ADDRESS_0,
                        operator: ADDRESS_1,
                        update: OperatorUpdate::Add,
                    }
                )
            ),
            "Incorrect event emitted"
        )
    }

    // Testing burn functionality
    #[concordium_test]
    fn test_burn() {
        let quantity_1 = ContractTokenAmount::from(1);
        let quantity_2 = ContractTokenAmount::from(2);
        
        // Setup the context
        let mut ctx = TestReceiveContext::empty();
        ctx.set_sender(ADDRESS_0);
        ctx.set_owner(ACCOUNT_0);
        ctx.set_invoker(ACCOUNT_0);

        // and parameter.
        let mint_data = new_mint_params(ADDRESS_0, ADDRESS_0, token_0(), quantity_2);

        let parameter_bytes = to_bytes(&mint_data);
        ctx.set_parameter(&parameter_bytes);

        let mut logger = TestLogger::init();
        let mut state_builder = TestStateBuilder::new();
        let state = State::empty(&mut state_builder);
        let mut host = TestHost::new(state, state_builder);

        // Call the contract function.
        let _: ContractResult<()> = mint(&ctx, &mut host, Amount::zero(), &mut logger);

        let parameter_bytes = to_bytes(&BurnParams {
            token_id: token_0(),
            quantity: quantity_1,
            owner: ADDRESS_0,
        });
        ctx.set_parameter(&parameter_bytes);

        // Call the contract function.
        let result: ContractResult<()> = burn(&ctx, &mut host, &mut logger);

        // Check the result
        claim!(result.is_ok(), "Results in rejection");

        // Check the logs
        claim!(
            logger.logs.contains(&to_bytes(&Cis2Event::Burn(BurnEvent {
                token_id: token_0(),
                amount: quantity_1,
                owner: ADDRESS_0
            }))),
            "Expected an event for buning by address ADDRESS_0"
        );

        // Check the state.
        let balance0 = host
            .state()
            .balance(&token_0(), &ADDRESS_0)
            .expect_report("Token is expected to exist");
        claim_eq!(
            balance0,
            quantity_1,
            "Token balance should be decreased by the burn quantity after first burn."
        );

        // Call the contract function.
        let _: ContractResult<()> = burn(&ctx, &mut host, &mut logger);

        // Check the state.
        claim!(
            host.state().balance(&token_0(), &ADDRESS_0).is_err(),
            "Token should be burn completely."
        );
    }

    // Testing update_price functionality
    #[concordium_test]
    fn test_update_price() {
        let quantity_1 = ContractTokenAmount::from(1);

        // Setup the context
        let mut ctx = TestReceiveContext::empty();
        ctx.set_sender(ADDRESS_0);
        ctx.set_owner(ACCOUNT_0);
        ctx.set_invoker(ACCOUNT_0);

        // and parameter.
        let mint_data = new_mint_params(ADDRESS_0, ADDRESS_0, token_0(), quantity_1);

        let parameter_bytes = to_bytes(&mint_data);
        ctx.set_parameter(&parameter_bytes);

        let mut logger = TestLogger::init();
        let mut state_builder = TestStateBuilder::new();
        let state = State::empty(&mut state_builder);
        let mut host = TestHost::new(state, state_builder);

        // Call the contract function.
        let _: ContractResult<()> = mint(&ctx, &mut host, Amount::zero(), &mut logger);

        let price = Amount::from_ccd(100);
        let update_price_params = UpdatePriceParameter {
            token_id: token_0(),
            price,
        };
        let parameter_bytes = to_bytes(&update_price_params);
        ctx.set_parameter(&parameter_bytes);

        // Call the contract function.
        let result: ContractResult<()> = update_price(&ctx, &mut host, &mut logger);

        // Check the result
        claim!(result.is_ok(), "Results in rejection");

        // Check the logs
        claim!(
            logger
                .logs
                .contains(&to_bytes(&CustomEvent::UpdatePrice(UpdatePriceEvent {
                    token_id: token_0(),
                    owner: ADDRESS_0,
                    from: Amount::zero(),
                    to: price
                }))),
            "Expected an event for buning by address ADDRESS_0"
        );
    }

    // Testing view_address_state_by_owner functionality
    #[concordium_test]
    fn test_view_address_state_by_owner() {
        let quantity_1 = ContractTokenAmount::from(1);

        // Setup the context
        let mut ctx = TestReceiveContext::empty();
        ctx.set_invoker(ACCOUNT_0);

        // and parameter.
        let mint_data = new_mint_params(ADDRESS_0, ADDRESS_0, token_0(), quantity_1);

        let parameter_bytes = to_bytes(&mint_data);
        ctx.set_parameter(&parameter_bytes);

        let mut logger = TestLogger::init();
        let mut state_builder = TestStateBuilder::new();
        let state = initial_state(&mut state_builder, quantity_1);
        let mut host = TestHost::new(state, state_builder);

        // Call the contract function.
        let _: ContractResult<()> = mint(&ctx, &mut host, Amount::zero(), &mut logger);

        let parameter_bytes = to_bytes(&ADDRESS_0);
        ctx.set_parameter(&parameter_bytes);

        // Call the contract function.
        let result: ReceiveResult<ViewAddressState> = view_address_state_by_owner(&ctx, &mut host);

        // Check the result
        claim!(result.is_ok(), "Results in rejection");
    }

    // Testing view_token functionality
    #[concordium_test]
    fn test_view_token() {
        let quantity_1 = ContractTokenAmount::from(1);

        // Setup the context
        let mut ctx = TestReceiveContext::empty();
        ctx.set_invoker(ACCOUNT_0);

        // and parameter.
        let mint_data = new_mint_params(ADDRESS_0, ADDRESS_0, token_0(), quantity_1);

        let parameter_bytes = to_bytes(&mint_data);
        ctx.set_parameter(&parameter_bytes);

        let mut logger = TestLogger::init();
        let mut state_builder = TestStateBuilder::new();
        let state = initial_state(&mut state_builder, quantity_1);
        let mut host = TestHost::new(state, state_builder);

        // Call the contract function.
        let _: ContractResult<()> = mint(&ctx, &mut host, Amount::zero(), &mut logger);

        let params = ViewTokenParams {
            token_id: token_0(),
            owner: ADDRESS_0,
        };
        let parameter_bytes = to_bytes(&params);
        ctx.set_parameter(&parameter_bytes);

        // Call the contract function.
        let result: ReceiveResult<NFTData> = view_token(&ctx, &mut host);

        // Check the result
        claim!(result.is_ok(), "Results in rejection");
    }
}
