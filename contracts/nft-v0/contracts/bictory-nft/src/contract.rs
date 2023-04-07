use super::*;

/// Initialize contract instance with no token types initially.
#[init(contract = "BictoryNFT")]
pub fn contract_init(_ctx: &impl HasInitContext) -> InitResult<State> {
    // Construct the initial contract state.
    let state = State::empty();
    Ok(state)
}

/// Mint new tokens with a given address as the owner of these tokens.
/// Logs a `Mint` and a `TokenMetadata` event for each token.
/// The url for the token metadata is the token ID encoded in hex, appended on
/// the `TOKEN_METADATA_BASE_URL`.
/// Owner and Minter will be same during minting.
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
    parameter = "MintingParameter",
    enable_logger,
    payable
)]
pub fn contract_mint<A: HasActions>(
    ctx: &impl HasReceiveContext,
    price: Amount,
    logger: &mut impl HasLogger,
    state: &mut State,
) -> ContractResult<A> {
    // Parse the parameter.
    let params: MintingParameter = ctx.parameter_cursor().get()?;
    let mut actions = A::accept();
    let owner = Address::Account(ctx.invoker());

    for mint_data in params.mint_data {
        let token_id = mint_data.token_id.clone();

        // Mint the token in the state.
        state.mint(owner, mint_data.clone(), price)?;

        let shares = calc_shares(price, mint_data.bictory_royalty as u64);

        if let Address::Account(creator) = mint_data.creator {
            actions = actions.and_then(A::simple_transfer(&creator, shares.creator));
        }

        if mint_data.bictory_royalty != 0 {
            let receive_name = ReceiveName::new_unchecked("BictoryConfig.sendCCD");

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
            actions = actions.and_then(send(
                &bictory_config_contract_address,
                receive_name,
                shares.bictory,
                &"",
            ));
        }

        // Event for minted NFT.
        logger.log(&Cis1Event::Mint(MintEvent {
            token_id: token_id.clone(),
            amount: 1,
            owner,
        }))?;

        // Metadata URL for the NFT.
        logger.log(&token_metadata_event(token_id))?;
    }

    Ok(actions)
}

/// Execute a list of token transfers, in the order of the list.
/// Considering first bit of `AdditionalData` as flagging of `for_sale`.
/// `0` or undefined means `for_sale` = false and for other than `0`
/// means `for_sale` = true.
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
    enable_logger
)]
pub fn contract_transfer<A: HasActions>(
    ctx: &impl HasReceiveContext,
    logger: &mut impl HasLogger,
    state: &mut State,
) -> ContractResult<A> {
    // Parse the parameter.
    let TransferParams(transfers): TransferParameter = ctx.parameter_cursor().get()?;
    // Get the sender who invoked this contract function.
    let sender = ctx.sender();

    let mut actions = A::accept();
    for transfer in transfers {
        // Authenticate the sender for this transfer
        ensure!(
            transfer.from == sender || state.is_operator(&sender, &transfer.from),
            ContractError::Unauthorized
        );

        let to_address = transfer.to.address();
        // Update the contract state
        state.transfer(&transfer)?;

        // Remove empty state
        state.clear_empty_state(&transfer.from);

        // Log transfer event
        logger.log(&Cis1Event::Transfer(TransferEvent {
            token_id: transfer.token_id.clone(),
            amount: transfer.amount,
            from: transfer.from,
            to: to_address,
        }))?;

        // If the receiver is a contract, we add sending it a message to the list of
        // actions.
        if let Receiver::Contract(address, function) = transfer.to {
            let parameter = OnReceivingCis1Params {
                token_id: transfer.token_id,
                amount: transfer.amount,
                from: transfer.from,
                contract_name: OwnedContractName::new_unchecked(String::from("init_BictoryNFT")),
                data: transfer.data,
            };
            let action = send(&address, function.as_ref(), Amount::zero(), &parameter);
            actions = actions.and_then(action);
        }
    }
    Ok(actions)
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
    enable_logger
)]
fn contract_update_operator<A: HasActions>(
    ctx: &impl HasReceiveContext,
    logger: &mut impl HasLogger,
    state: &mut State,
) -> ContractResult<A> {
    // Parse the parameter.
    let UpdateOperatorParams(params) = ctx.parameter_cursor().get()?;
    // Get the sender who invoked this contract function.
    let sender = Address::Account(ctx.invoker());

    for param in params {
        // Update the operator in the state.
        match param.update {
            OperatorUpdate::Add => state.add_operator(&sender, &param.operator),
            OperatorUpdate::Remove => state.remove_operator(&sender, &param.operator),
        }

        // Log the appropriate event
        logger.log(&Cis1Event::<ContractTokenId>::UpdateOperator(
            UpdateOperatorEvent {
                owner: sender,
                operator: param.operator,
                update: param.update,
            },
        ))?;
    }

    Ok(A::accept())
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
    parameter = "OperatorOfQueryParams"
)]
fn contract_operator_of<A: HasActions>(
    ctx: &impl HasReceiveContext,
    state: &mut State,
) -> ContractResult<A> {
    // Parse the parameter.
    let params: OperatorOfQueryParams = ctx.parameter_cursor().get()?;
    // Build the response.
    let mut response = Vec::with_capacity(params.queries.len());
    for query in params.queries {
        // Query the state for address being an operator of owner.
        let is_operator = state.is_operator(&query.owner, &query.address);
        response.push((query, is_operator));
    }
    // Send back the response.
    Ok(send(
        &params.result_contract,
        params.result_function.as_ref(),
        Amount::zero(),
        &OperatorOfQueryResponse::from(response),
    ))
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
    parameter = "ContractBalanceOfQueryParams"
)]
fn contract_balance_of<A: HasActions>(
    ctx: &impl HasReceiveContext,
    state: &mut State,
) -> ContractResult<A> {
    // Parse the parameter.
    let params: ContractBalanceOfQueryParams = ctx.parameter_cursor().get()?;
    // Build the response.
    let mut response = Vec::with_capacity(params.queries.len());
    for query in params.queries {
        // Query the state for balance.
        let amount = state.balance(&query.token_id, &query.address)?;
        response.push((query, amount));
    }
    // Send back the response.
    Ok(send(
        &params.result_contract,
        params.result_function.as_ref(),
        Amount::zero(),
        &BalanceOfQueryResponse::from(response),
    ))
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
    parameter = "ContractTokenId",
    enable_logger
)]
pub fn contract_burn<A: HasActions>(
    ctx: &impl HasReceiveContext,
    logger: &mut impl HasLogger,
    state: &mut State,
) -> ContractResult<A> {
    // Parse the parameter.
    let token_id: ContractTokenId = ctx.parameter_cursor().get()?;

    let owner = Address::Account(ctx.invoker());

    // Burning NFT
    let event = state.burn(&owner, token_id.clone())?;

    // Remove empty state
    state.clear_empty_state(&owner);

    // Event for burning NFT.
    logger.log(&Cis1Event::Burn(event))?;

    // Metadata URL for the NFT.
    logger.log(&token_metadata_event(token_id))?;

    Ok(A::accept())
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
    enable_logger
)]
pub fn contract_update_price<A: HasActions>(
    ctx: &impl HasReceiveContext,
    logger: &mut impl HasLogger,
    state: &mut State,
) -> ContractResult<A> {
    // Parse the parameter.
    let params: UpdatePriceParameter = ctx.parameter_cursor().get()?;

    let token_id = params.token_id.clone();

    // Updating Price
    let event = state.update_price(&Address::Account(ctx.invoker()), params)?;

    // Event for updating price of NFT.
    logger.log(&CustomEvent::UpdatePrice(event))?;

    // Metadata URL for the NFT.
    logger.log(&token_metadata_event(token_id))?;

    Ok(A::accept())
}

fn token_metadata_event(token_id: ContractTokenId) -> Cis1Event<ContractTokenId> {
    let token_metadata_url = build_token_metadata_url(&token_id);
    Cis1Event::TokenMetadata(TokenMetadataEvent {
        token_id,
        metadata_url: MetadataUrl {
            url: token_metadata_url,
            hash: None,
        },
    })
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

    fn get_mint_data(creator: Address, token_id: ContractTokenId) -> MintData<ContractTokenId> {
        MintData {
            token_id,
            cid: Vec::new(),
            creator_royalty: 0,
            minter_royalty: 0,
            creator,
            bictory_royalty: 0,
        }
    }

    fn new_mint_params(creator: Address, token_id: ContractTokenId) -> MintingParameter {
        let mut mint_data_set = Set::default();
        mint_data_set.insert(get_mint_data(creator, token_id));

        MintParams {
            mint_data: mint_data_set,
        }
    }

    /// Test helper function which creates a contract state with two tokens with
    /// id `token_0` and id `token_1` owned by `ADDRESS_0`
    fn initial_state() -> State {
        let mut state = State::empty();
        let price = Amount::zero();
        // parameter
        let mut mint_data = get_mint_data(ADDRESS_0, token_0());

        state
            .mint(ADDRESS_0, mint_data, price)
            .expect_report("Failed to mint token_0");

        // parameter
        mint_data = get_mint_data(ADDRESS_1, token_1());

        state
            .mint(ADDRESS_1, mint_data, price)
            .expect_report("Failed to mint token_1");

        state
    }

    /// Test initialization succeeds.
    #[concordium_test]
    fn test_init() {
        // Setup the context
        let ctx = InitContextTest::empty();

        // Call the contract function.
        let result = contract_init(&ctx);

        // Check the result
        let _ = result.expect_report("Contract initialization failed");
    }

    /// Test minting, ensuring the new tokens are owned by the given address and
    /// the appropriate events are logged.
    #[concordium_test]
    fn test_mint() {
        // Setup the context
        let mut ctx = ReceiveContextTest::empty();
        ctx.set_sender(ADDRESS_0);
        ctx.set_owner(ACCOUNT_0);
        ctx.set_invoker(ACCOUNT_0);

        let mint_data = new_mint_params(ADDRESS_0, token_0());

        let parameter_bytes = to_bytes(&mint_data);
        ctx.set_parameter(&parameter_bytes);

        let mut logger = LogRecorder::init();
        let mut state = State::empty();

        // Call the contract function.
        let result: ContractResult<ActionsTree> =
            contract_mint(&ctx, Amount::zero(), &mut logger, &mut state);

        // Check the result
        let _ = result.expect_report("Results in rejection");

        // Check the logs
        claim!(
            logger.logs.contains(&to_bytes(&Cis1Event::Mint(MintEvent {
                owner: ADDRESS_0,
                token_id: token_0(),
                amount: 1,
            }))),
            "Expected an event for minting token_0"
        );
    }

    /// Test transfer succeeds, when `from` is the sender.
    #[concordium_test]
    fn test_transfer_account() {
        // Setup the context
        let mut ctx = ReceiveContextTest::empty();
        ctx.set_sender(ADDRESS_0);

        // and parameter.
        let transfer = Transfer {
            token_id: token_0(),
            from: ADDRESS_0,
            to: Receiver::from_account(ACCOUNT_1),
            amount: 1,
            data: AdditionalData::empty(),
        };
        let parameter = TransferParams(vec![transfer]);
        let parameter_bytes = to_bytes(&parameter);
        ctx.set_parameter(&parameter_bytes);

        let mut logger = LogRecorder::init();
        let mut state = initial_state();

        // Call the contract function.
        let result: ContractResult<ActionsTree> = contract_transfer(&ctx, &mut logger, &mut state);
        // Check the result.
        let actions = result.expect_report("Results in rejection");
        claim_eq!(
            actions,
            ActionsTree::accept(),
            "No action should be produced."
        );

        // Check the logs.
        claim_eq!(logger.logs.len(), 1, "Only one event should be logged");
        claim_eq!(
            logger.logs[0],
            to_bytes(&Cis1Event::Transfer(TransferEvent {
                from: ADDRESS_0,
                to: ADDRESS_1,
                token_id: token_0(),
                amount: 1,
            })),
            "Incorrect event emitted"
        )
    }

    /// Test transfer token fails, when sender is neither the owner or an
    /// operator of the owner.
    #[concordium_test]
    fn test_transfer_not_authorized() {
        // Setup the context
        let mut ctx = ReceiveContextTest::empty();
        ctx.set_sender(ADDRESS_1);

        // and parameter.
        let transfer = Transfer {
            token_id: token_0(),
            from: ADDRESS_0,
            amount: 1,
            to: Receiver::from_account(ACCOUNT_1),
            data: AdditionalData::empty(),
        };
        let parameter = TransferParams(vec![transfer]);
        let parameter_bytes = to_bytes(&parameter);
        ctx.set_parameter(&parameter_bytes);

        let mut logger = LogRecorder::init();
        let mut state = initial_state();

        // Call the contract function.
        let result: ContractResult<ActionsTree> = contract_transfer(&ctx, &mut logger, &mut state);
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
        // Setup the context
        let mut ctx = ReceiveContextTest::empty();
        ctx.set_sender(ADDRESS_1);

        // and parameter.
        let transfer = Transfer {
            from: ADDRESS_0,
            to: Receiver::from_account(ACCOUNT_1),
            token_id: token_0(),
            amount: 1,
            data: AdditionalData::empty(),
        };
        let parameter = TransferParams::from(vec![transfer]);
        let parameter_bytes = to_bytes(&parameter);
        ctx.set_parameter(&parameter_bytes);

        let mut logger = LogRecorder::init();
        let mut state = initial_state();
        state.add_operator(&ADDRESS_0, &ADDRESS_1);

        // Call the contract function.
        let result: ContractResult<ActionsTree> = contract_transfer(&ctx, &mut logger, &mut state);

        // Check the result.
        let actions: ActionsTree = result.expect_report("Results in rejection");
        claim_eq!(
            actions,
            ActionsTree::accept(),
            "No action should be produced."
        );

        // Check the state.
        let balance0 = state
            .balance(&token_0(), &ADDRESS_0)
            .expect_report("Token is expected to exist");
        let balance1 = state
            .balance(&token_0(), &ADDRESS_1)
            .expect_report("Token is expected to exist");
        claim_eq!(
            balance0,
            0,
            "Token owner balance should be decreased by the transferred amount"
        );
        claim_eq!(
            balance1,
            1,
            "Token receiver balance should be increased by the transferred amount"
        );

        // Check the logs.
        claim_eq!(logger.logs.len(), 1, "Only one event should be logged");
        claim_eq!(
            logger.logs[0],
            to_bytes(&Cis1Event::Transfer(TransferEvent {
                from: ADDRESS_0,
                to: ADDRESS_1,
                token_id: token_0(),
                amount: 1,
            })),
            "Incorrect event emitted"
        )
    }

    /// Test adding an operator succeeds and the appropriate event is logged.
    #[concordium_test]
    fn test_add_operator() {
        // Setup the context
        let mut ctx = ReceiveContextTest::empty();
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

        let mut logger = LogRecorder::init();
        let mut state = initial_state();

        // Call the contract function.
        let result: ContractResult<ActionsTree> =
            contract_update_operator(&ctx, &mut logger, &mut state);

        // Check the result.
        let actions: ActionsTree = result.expect_report("Results in rejection");
        claim_eq!(
            actions,
            ActionsTree::accept(),
            "No action should be produced."
        );

        // Check the state.
        let is_operator = state.is_operator(&ADDRESS_1, &ADDRESS_0);
        claim!(is_operator, "Account should be an operator");

        // Check the logs.
        claim_eq!(logger.logs.len(), 1, "One event should be logged");
        claim_eq!(
            logger.logs[0],
            to_bytes(&Cis1Event::<ContractTokenId>::UpdateOperator(
                UpdateOperatorEvent {
                    owner: ADDRESS_0,
                    operator: ADDRESS_1,
                    update: OperatorUpdate::Add,
                }
            )),
            "Incorrect event emitted"
        )
    }

    // Testing burn functionality
    #[concordium_test]
    fn test_burn() {
        // Setup the context
        let mut ctx = ReceiveContextTest::empty();
        ctx.set_sender(ADDRESS_0);
        ctx.set_owner(ACCOUNT_0);
        ctx.set_invoker(ACCOUNT_0);

        // and parameter.
        let mint_data = new_mint_params(ADDRESS_0, token_0());

        let parameter_bytes = to_bytes(&mint_data);
        ctx.set_parameter(&parameter_bytes);

        let mut logger = LogRecorder::init();
        let mut state = State::empty();

        // Call the contract function.
        let _: ContractResult<ActionsTree> =
            contract_mint(&ctx, Amount::zero(), &mut logger, &mut state);

        let parameter_bytes = to_bytes(&token_0());
        ctx.set_parameter(&parameter_bytes);

        // Call the contract function.
        let result: ContractResult<ActionsTree> = contract_burn(&ctx, &mut logger, &mut state);

        // Check the result
        let actions = result.expect_report("Results in rejection");
        claim_eq!(
            actions,
            ActionsTree::accept(),
            "No action should be produced."
        );

        // Check the logs
        claim!(
            logger.logs.contains(&to_bytes(&Cis1Event::Burn(BurnEvent {
                token_id: token_0(),
                amount: 1,
                owner: ADDRESS_0
            }))),
            "Expected an event for buning by address ADDRESS_0"
        );
    }

    // Testing update_price functionality
    #[concordium_test]
    fn test_update_price() {
        // Setup the context
        let mut ctx = ReceiveContextTest::empty();
        ctx.set_sender(ADDRESS_0);
        ctx.set_owner(ACCOUNT_0);
        ctx.set_invoker(ACCOUNT_0);

        // and parameter.
        let mint_data = new_mint_params(ADDRESS_0, token_0());

        let parameter_bytes = to_bytes(&mint_data);
        ctx.set_parameter(&parameter_bytes);

        let mut logger = LogRecorder::init();
        let mut state = State::empty();

        // Call the contract function.
        let _: ContractResult<ActionsTree> =
            contract_mint(&ctx, Amount::zero(), &mut logger, &mut state);

        let update_price_params = UpdatePriceParameter {
            token_id: token_0(),
            price: Amount::from_ccd(100),
        };
        let parameter_bytes = to_bytes(&update_price_params);
        ctx.set_parameter(&parameter_bytes);

        // Call the contract function.
        let result: ContractResult<ActionsTree> =
            contract_update_price(&ctx, &mut logger, &mut state);

        // Check the result
        let actions = result.expect_report("Results in rejection");
        claim_eq!(
            actions,
            ActionsTree::accept(),
            "No action should be produced."
        );

        // Check the logs
        claim!(
            logger
                .logs
                .contains(&to_bytes(&CustomEvent::UpdatePrice(UpdatePriceEvent {
                    token_id: token_0(),
                    owner: ADDRESS_0,
                    from: 0,
                    to: 100_000_000
                }))),
            "Expected an event for buning by address ADDRESS_0"
        );
    }
}
