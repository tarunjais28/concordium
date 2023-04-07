use super::*;

/// Initialize contract instance with no token types initially.
#[init(contract = "BictoryNFT", parameter = "ContractAddress")]
fn contract_init(ctx: &impl HasInitContext) -> InitResult<State> {
    let storage_address: ContractAddress = ctx.parameter_cursor().get()?;
    // Construct the initial contract state.
    let state = State::new(storage_address);
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
#[receive(contract = "BictoryNFT", name = "mint", parameter = "MintingParameter")]
fn contract_mint<A: HasActions>(
    ctx: &impl HasReceiveContext,
    state: &mut State,
) -> ContractResult<A> {
    // Ensure that no other call is in progress
    ensure!(
        state.function_state.is_none(),
        CustomContractError::RequestInProgress.into()
    );

    let storage = StorageContract::new(&state.storage_address);

    // Parse the parameter.
    let params: MintingParameter = ctx.parameter_cursor().get()?;

    let mut mint_state = Vec::new();

    let mut actions = A::accept();

    for mint_data in params.mint_data {
        let token_id = mint_data.token_id;

        // Ensuring correct hash
        ensure!(
            token_id.0.len() == 32,
            CustomContractError::InvalidHash.into()
        );

        actions = actions.and_then(storage.send_find(
            &ctx.self_address(),
            "BictoryNFT.functionContinue",
            <&ByteSlice>::from(token_id.0.as_slice()),
        ));

        let mint_data = structs::MintData {
            token_id,
            creator: mint_data.creator,
            creator_royalty: mint_data.creator_royalty,
            minter: Address::Account(ctx.invoker()),
            minter_royalty: mint_data.minter_royalty,
            price: mint_data.price,
            cid: mint_data.cid,
        };
        mint_state.push(mint_data);
    }

    state.function_state = Some(FunctionState::Mint(mint_state));

    Ok(actions)
}

#[receive(contract = "BictoryNFT", name = "functionContinue", enable_logger)]
fn contract_continue<A: HasActions>(
    ctx: &impl HasReceiveContext,
    logger: &mut impl HasLogger,
    state: &mut State,
) -> ContractResult<A> {
    if let Address::Account(_) = ctx.sender() {
        return Err(CustomContractError::ContractOnly.into());
    };

    let storage = StorageContract::new(&state.storage_address);

    let function_state = state
        .function_state
        .take()
        .ok_or(CustomContractError::NoRequestInProgress)?;

    match function_state {
        FunctionState::Mint(mut mint_state) => {
            let params = StorageFindResponse::deserial(&mut ctx.parameter_cursor())?;

            ensure!(
                params.contract.is_none(),
                CustomContractError::TokenIdAlreadyExists.into()
            );

            let token = if let Some(idx) = mint_state
                .iter()
                .position(|token_data| token_data.token_id.0 == params.prefix.0)
            {
                let token = mint_state.remove(idx);

                // If all mints handled, clear function_state
                if mint_state.is_empty() {
                    state.function_state = None;
                } else {
                    state.function_state = Some(FunctionState::Mint(mint_state));
                }

                token
            } else {
                return Err(CustomContractError::UnknownToken.into());
            };

            let action = storage.send_set(
                <&ByteSlice>::from(token.token_id.0.as_slice()),
                &[
                    StorageEntryRef::new(OWNER, &token.minter),
                    StorageEntryRef::new(CREATOR, &token.minter),
                    StorageEntryRef::new(CREATOR_ROYALTY, &token.creator_royalty),
                    StorageEntryRef::new(MINTER, &token.minter),
                    StorageEntryRef::new(MINTER_ROYALTY, &token.minter_royalty),
                    StorageEntryRef::new(PRICE, &token.price),
                    StorageEntryRef::new(CID, &token.cid),
                ],
            );

            // Event for minted NFT.
            logger.log(&Cis1Event::Mint(MintEvent {
                token_id: token.token_id.clone(),
                amount: token.price.micro_ccd,
                owner: token.minter,
            }))?;

            // Metadata URL for the NFT.
            logger.log(&token_metadata_event(token.token_id))?;

            Ok(action)
        }
        FunctionState::Transfer(mut transfer_state) => {
            let params = StorageFindResponse::deserial(&mut ctx.parameter_cursor())?;

            let leaf = if let Some(leaf) = params.contract {
                leaf
            } else {
                return Err(Cis1Error::InvalidTokenId.into());
            };

            let transfer = if let Some(idx) = transfer_state
                .iter()
                .position(|transfer| transfer.token_id.0 == params.prefix.0)
            {
                let transfer = transfer_state.remove(idx);
                // If all transfers handled, clear function_state
                if transfer_state.is_empty() {
                    state.function_state = None;
                } else {
                    state.function_state = Some(FunctionState::Transfer(transfer_state));
                }
                transfer
            } else {
                return Err(CustomContractError::UnknownToken.into());
            };

            if transfer.amount == 0 {
                return Ok(A::accept());
            } else if transfer.amount > 1 {
                return Err(ContractError::InsufficientFunds);
            }

            let mut actions = A::accept();

            // If the receiver is a contract, we add sending it a message to the list of
            // actions.
            let to_address = match transfer.to {
                Receiver::Contract(address, function) => {
                    let parameter = OnReceivingCis1Params {
                        token_id: transfer.token_id.clone(),
                        amount: transfer.amount,
                        from: transfer.from,
                        contract_name: OwnedContractName::new_unchecked(String::from(
                            "init_BictoryNFT",
                        )),
                        data: transfer.data,
                    };
                    let action = send(&address, function.as_ref(), Amount::zero(), &parameter);
                    actions = actions.and_then(action);
                    Address::Contract(address)
                }
                Receiver::Account(address) => Address::Account(address),
            };

            // Update owner field
            actions = actions.and_then(storage.send_set(
                transfer.token_id.0.as_slice(),
                &[StorageEntryRef::new(OWNER, &to_address)],
            ));

            // FIXME: ALL operators must be removed on token transfer. Otherwise malicious contract controlled by
            // previous owner may be kept as a token operator! This requires storage API change
            // TODO: Leaf contract also requires proper iterface. See `StorageContract`
            // Update leaf contract rights
            actions = actions.and_then(send(
                &leaf,
                ReceiveName::new_unchecked("BictoryStorage.updateOperator"),
                Amount::zero(),
                &UpdateOperatorParams(vec![
                    UpdateOperator {
                        update: OperatorUpdate::Add,
                        operator: to_address,
                    },
                    UpdateOperator {
                        update: OperatorUpdate::Remove,
                        operator: transfer.from,
                    },
                ]),
            ));

            // Log transfer event
            logger.log(&Cis1Event::Transfer(TransferEvent {
                token_id: transfer.token_id.clone(),
                amount: transfer.amount,
                from: transfer.from,
                to: to_address,
            }))?;

            // Metadata URL for the NFT.
            logger.log(&token_metadata_event(transfer.token_id))?;

            Ok(actions)
        }
        FunctionState::UpdatePrice(price_update) => {
            let params = StorageGetResponse::deserial(&mut ctx.parameter_cursor())?;

            let price: Amount = params.get(PRICE)?;
            let owner = params.get(OWNER)?;

            // Log Update Price event
            logger.log(&CustomEvent::UpdatePrice(UpdatePriceEvent {
                token_id: price_update.token_id.clone(),
                owner,
                from: price.micro_ccd,
                to: price_update.price.micro_ccd,
            }))?;

            // Metadata URL for the NFT.
            logger.log(&token_metadata_event(price_update.token_id.clone()))?;

            Ok(storage.send_set(
                &Bytes(price_update.token_id.0),
                &[StorageEntryRef::new(PRICE, &price_update.price)],
            ))
        }
        FunctionState::Burn(step) => {
            match step {
                BurnStep::Find(token_id) => {
                    let params = StorageFindResponse::deserial(&mut ctx.parameter_cursor())?;
                    if let Some(leaf) = params.contract {
                        state.function_state = Some(FunctionState::Burn(BurnStep::GetInfo(leaf)));
                        Ok(send(
                            &leaf,
                            ReceiveName::new_unchecked("BictoryStorage.get"),
                            Amount::zero(),
                            &StorageGetParams {
                                result_contract: ctx.self_address(),
                                result_function: OwnedReceiveName::new_unchecked(String::from(
                                    "BictoryNFT.functionContinue",
                                )),
                                prefix: Bytes(token_id.0),
                                keys: vec![
                                    Bytes(OWNER.as_bytes().to_vec()),
                                    Bytes(PRICE.as_bytes().to_vec()),
                                ],
                            },
                        ))
                    } else {
                        return Err(CustomContractError::UnknownToken.into());
                    }
                }
                BurnStep::GetInfo(_) => {
                    let params = StorageGetResponse::deserial(&mut ctx.parameter_cursor())?;
                    let owner = params.get(OWNER)?;
                    let price = params.get(PRICE)?;

                    // FIXME: Interact with leaf instead of root
                    // FIXME: Remove all operators on the leaf to prevent updating NFT data after burn
                    let actions = storage.send_unset(&params.prefix, &[<&ByteSlice>::from(OWNER)]);

                    // Log Burn event
                    logger.log(&Cis1Event::Burn(BurnEvent {
                        token_id: TokenIdVec(params.prefix.0.clone()),
                        owner,
                        amount: price,
                    }))?;

                    // Metadata URL for the NFT.
                    logger.log(&token_metadata_event(TokenIdVec(params.prefix.0)))?;

                    Ok(actions)
                }
            }
        }
    }
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
    parameter = "TransferParameter"
)]
fn contract_transfer<A: HasActions>(
    ctx: &impl HasReceiveContext,
    state: &mut State,
) -> ContractResult<A> {
    ensure!(
        state.function_state.is_none(),
        CustomContractError::RequestInProgress.into()
    );

    let storage = StorageContract::new(&state.storage_address);

    // Parse the parameter.
    let TransferParams(transfers) = TransferParameter::deserial(&mut ctx.parameter_cursor())?;

    let invoker = Address::Account(ctx.invoker());

    let mut actions = A::accept();
    let self_address = ctx.self_address();

    for transfer in &transfers {
        // Authenticate the invoker for this transfer
        ensure!(transfer.from == invoker, ContractError::Unauthorized);

        actions = actions.and_then(storage.send_find(
            &self_address,
            "BictoryNFT.functionContinue",
            <&ByteSlice>::from(transfer.token_id.0.as_slice()),
        ));
    }

    state.function_state = Some(FunctionState::Transfer(transfers));

    Ok(actions)
}

// /// Enable or disable addresses as operators of the sender address.
// /// Logs an `UpdateOperator` event.
// ///
// /// It rejects if:
// /// - It fails to parse the parameter.
// /// - The operator address is the same as the sender address.
// /// - Fails to log event.
// #[receive(
//     contract = "BictoryNFT",
//     name = "updateOperator",
//     parameter = "UpdateOperatorParams",
//     enable_logger
// )]
// fn contract_update_operator<A: HasActions>(
//     ctx: &impl HasReceiveContext,
//     logger: &mut impl HasLogger,
//     state: &mut State,
// ) -> ContractResult<A> {
//     // Parse the parameter.
//     let UpdateOperatorParams(params) = ctx.parameter_cursor().get()?;
//     // Get the sender who invoked this contract function.
//     let sender = ctx.sender();

//     for param in params {
//         // Update the operator in the state.
//         match param.update {
//             OperatorUpdate::Add => state.add_operator(&sender, &param.operator),
//             OperatorUpdate::Remove => state.remove_operator(&sender, &param.operator),
//         }

//         // Log the appropriate event
//         logger.log(&Cis1Event::<ContractTokenId>::UpdateOperator(
//             UpdateOperatorEvent {
//                 owner: sender,
//                 operator: param.operator,
//                 update: param.update,
//             },
//         ))?;
//     }

//     Ok(A::accept())
// }

// /// Takes a list of queries. Each query is an owner address and some address to
// /// check as an operator of the owner address. It takes a contract address plus
// /// contract function to invoke with the result.
// ///
// /// It rejects if:
// /// - It fails to parse the parameter.
// /// - Message sent back with the result rejects.
// #[receive(
//     contract = "BictoryNFT",
//     name = "operatorOf",
//     parameter = "OperatorOfQueryParams"
// )]
// fn contract_operator_of<A: HasActions>(
//     ctx: &impl HasReceiveContext,
//     state: &mut State,
// ) -> ContractResult<A> {
//     // Parse the parameter.
//     let params: OperatorOfQueryParams = ctx.parameter_cursor().get()?;
//     // Build the response.
//     let mut response = Vec::with_capacity(params.queries.len());
//     for query in params.queries {
//         // Query the state for address being an operator of owner.
//         let is_operator = state.is_operator(&query.owner, &query.address);
//         response.push((query, is_operator));
//     }
//     // Send back the response.
//     Ok(send(
//         &params.result_contract,
//         params.result_function.as_ref(),
//         Amount::zero(),
//         &OperatorOfQueryResponse::from(response),
//     ))
// }

// /// Get the balance of given token IDs and addresses. It takes a contract
// /// address plus contract function to invoke with the result.
// ///
// /// It rejects if:
// /// - It fails to parse the parameter.
// /// - Any of the queried `token_id` does not exist.
// /// - Message sent back with the result rejects.
// #[receive(
//     contract = "BictoryNFT",
//     name = "balanceOf",
//     parameter = "ContractBalanceOfQueryParams"
// )]
// fn contract_balance_of<A: HasActions>(
//     ctx: &impl HasReceiveContext,
//     state: &mut State,
// ) -> ContractResult<A> {
//     // Parse the parameter.
//     let params: ContractBalanceOfQueryParams = ctx.parameter_cursor().get()?;
//     // Build the response.
//     let mut response = Vec::with_capacity(params.queries.len());
//     for query in params.queries {
//         // Query the state for balance.
//         let amount = state.balance(&query.token_id, &query.address)?;
//         response.push((query, amount));
//     }
//     // Send back the response.
//     Ok(send(
//         &params.result_contract,
//         params.result_function.as_ref(),
//         Amount::zero(),
//         &BalanceOfQueryResponse::from(response),
//     ))
// }

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
#[receive(contract = "BictoryNFT", name = "burn", parameter = "ContractTokenId")]
fn contract_burn<A: HasActions>(
    ctx: &impl HasReceiveContext,
    state: &mut State,
) -> ContractResult<A> {
    ensure!(
        state.function_state.is_none(),
        CustomContractError::RequestInProgress.into()
    );

    // Parse the parameter.
    let token_id: ContractTokenId = ctx.parameter_cursor().get()?;

    state.function_state = Some(FunctionState::Burn(BurnStep::Find(token_id.clone())));

    Ok(send(
        &state.storage_address,
        ReceiveName::new_unchecked("BictoryStorage.get"),
        Amount::zero(),
        &StorageFindParams {
            result_contract: ctx.self_address(),
            result_function: OwnedReceiveName::new_unchecked(
                "BictoryNFT.functionContinue".to_string(),
            ),
            prefix: Bytes(token_id.0),
        },
    ))
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
    parameter = "UpdatePriceParameter"
)]
fn contract_update_price<A: HasActions>(
    ctx: &impl HasReceiveContext,
    state: &mut State,
) -> ContractResult<A> {
    ensure!(
        state.function_state.is_none(),
        CustomContractError::RequestInProgress.into()
    );
    // Parse the parameter.
    let params: UpdatePriceParameter = ctx.parameter_cursor().get()?;

    Ok(send(
        &state.storage_address,
        ReceiveName::new_unchecked("BictoryStorage.set"),
        Amount::zero(),
        &StorageSetParams {
            prefix: Bytes(params.token_id.0),
            data: vec![StorageEntry {
                key: Bytes(PRICE.as_bytes().to_vec()),
                value: Bytes(to_bytes(&params.price)),
            }],
        },
    ))
}

// #[concordium_cfg_test]
// mod tests {
//     use super::*;
//     use test_infrastructure::*;

//     const ACCOUNT_0: AccountAddress = AccountAddress([0u8; 32]);
//     const ADDRESS_0: Address = Address::Account(ACCOUNT_0);
//     const ACCOUNT_1: AccountAddress = AccountAddress([1u8; 32]);
//     const ADDRESS_1: Address = Address::Account(ACCOUNT_1);
//     const STORAGE_ADDRESS: ContractAddress = ContractAddress {
//         index: 0,
//         subindex: 0,
//     };

//     fn token_0() -> ContractTokenId {
//         TokenIdVec(vec![0, 1])
//     }
//     fn token_1() -> ContractTokenId {
//         TokenIdVec(vec![42, 84, 168])
//     }

//     fn new_mint_params(owner: Address, token_id: ContractTokenId) -> MintingParameter {
//         let mut mint_data_set = Set::default();
//         mint_data_set.insert(MintData {
//             token_id,
//             price: Amount::zero(),
//             cid: Vec::new(),
//             hash: Vec::new(),
//             creator_royalty: 0,
//             minter_royalty: 0,
//             creator: owner,
//         });

//         MintParams {
//             mint_data: mint_data_set,
//         }
//     }

//     /// Test helper function which creates a contract state with two tokens with
//     /// id `token_0` and id `token_1` owned by `ADDRESS_0`
//     fn initial_state() -> State {
//         let mut state = State::new(STORAGE_ADDRESS);

//         // parameter
//         let mint_data = MintData {
//             token_id: token_0(),
//             price: Amount::zero(),
//             cid: Vec::new(),
//             hash: Vec::new(),
//             creator_royalty: 0,
//             minter_royalty: 0,
//             creator: ADDRESS_0,
//         };

//         state
//             .mint(ADDRESS_0, mint_data)
//             .expect_report("Failed to mint token_0");

//         // parameter
//         let mint_data = MintData {
//             token_id: token_1(),
//             price: Amount::zero(),
//             cid: Vec::new(),
//             hash: Vec::new(),
//             creator_royalty: 0,
//             minter_royalty: 0,
//             creator: ADDRESS_1,
//         };

//         state
//             .mint(ADDRESS_1, mint_data)
//             .expect_report("Failed to mint token_1");

//         state
//     }

//     /// Test initialization succeeds.
//     #[concordium_test]
//     fn test_init() {
//         // Setup the context
//         let ctx = InitContextTest::empty();

//         // Call the contract function.
//         let result = contract_init(&ctx);

//         // Check the result
//         let state = result.expect_report("Contract initialization failed");

//         // Check the state
//         claim_eq!(state.all_tokens.len(), 0, "No token should be initialized");
//     }

//     /// Test minting, ensuring the new tokens are owned by the given address and
//     /// the appropriate events are logged.
//     #[concordium_test]
//     fn test_mint() {
//         // Setup the context
//         let mut ctx = ReceiveContextTest::empty();
//         ctx.set_sender(ADDRESS_0);
//         ctx.set_owner(ACCOUNT_0);
//         ctx.set_invoker(ACCOUNT_0);

//         let mint_data = new_mint_params(ADDRESS_0, token_0());

//         let parameter_bytes = to_bytes(&mint_data);
//         ctx.set_parameter(&parameter_bytes);

//         let mut logger = LogRecorder::init();
//         let mut state = State::new(STORAGE_ADDRESS);

//         // Call the contract function.
//         let result: ContractResult<ActionsTree> = contract_mint(&ctx, &mut logger, &mut state);

//         // Check the result
//         let actions = result.expect_report("Results in rejection");
//         claim_eq!(
//             actions,
//             ActionsTree::accept(),
//             "No action should be produced."
//         );

//         // Check the state
//         claim_eq!(
//             state.all_tokens.len(),
//             1,
//             "Expected one tokens in the state."
//         );

//         // Check the logs
//         claim!(
//             logger.logs.contains(&to_bytes(&Cis1Event::Mint(MintEvent {
//                 owner: ADDRESS_0,
//                 token_id: token_0(),
//                 amount: 0,
//             }))),
//             "Expected an event for minting token_0"
//         );
//     }

//     /// Test transfer succeeds, when `from` is the sender.
//     #[concordium_test]
//     fn test_transfer_account() {
//         // Setup the context
//         let mut ctx = ReceiveContextTest::empty();
//         ctx.set_sender(ADDRESS_0);

//         // and parameter.
//         let transfer = Transfer {
//             token_id: token_0(),
//             from: ADDRESS_0,
//             to: Receiver::from_account(ACCOUNT_1),
//             amount: 1,
//             data: AdditionalData::empty(),
//         };
//         let parameter = TransferParams(vec![transfer]);
//         let parameter_bytes = to_bytes(&parameter);
//         ctx.set_parameter(&parameter_bytes);

//         let mut logger = LogRecorder::init();
//         let mut state = initial_state();

//         // Call the contract function.
//         let result: ContractResult<ActionsTree> = contract_transfer(&ctx, &mut logger, &mut state);
//         // Check the result.
//         let actions = result.expect_report("Results in rejection");
//         claim_eq!(
//             actions,
//             ActionsTree::accept(),
//             "No action should be produced."
//         );

//         // Check the logs.
//         claim_eq!(logger.logs.len(), 1, "Only one event should be logged");
//         claim_eq!(
//             logger.logs[0],
//             to_bytes(&Cis1Event::Transfer(TransferEvent {
//                 from: ADDRESS_0,
//                 to: ADDRESS_1,
//                 token_id: token_0(),
//                 amount: 1,
//             })),
//             "Incorrect event emitted"
//         )
//     }

//     /// Test transfer token fails, when sender is neither the owner or an
//     /// operator of the owner.
//     #[concordium_test]
//     fn test_transfer_not_authorized() {
//         // Setup the context
//         let mut ctx = ReceiveContextTest::empty();
//         ctx.set_sender(ADDRESS_1);

//         // and parameter.
//         let transfer = Transfer {
//             token_id: token_0(),
//             from: ADDRESS_0,
//             amount: 1,
//             to: Receiver::from_account(ACCOUNT_1),
//             data: AdditionalData::empty(),
//         };
//         let parameter = TransferParams(vec![transfer]);
//         let parameter_bytes = to_bytes(&parameter);
//         ctx.set_parameter(&parameter_bytes);

//         let mut logger = LogRecorder::init();
//         let mut state = initial_state();

//         // Call the contract function.
//         let result: ContractResult<ActionsTree> = contract_transfer(&ctx, &mut logger, &mut state);
//         // Check the result.
//         let err = result.expect_err_report("Expected to fail");
//         claim_eq!(
//             err,
//             ContractError::Unauthorized,
//             "Error is expected to be Unauthorized"
//         )
//     }

//     /// Test transfer succeeds when sender is not the owner, but is an operator
//     /// of the owner.
//     #[concordium_test]
//     fn test_operator_transfer() {
//         // Setup the context
//         let mut ctx = ReceiveContextTest::empty();
//         ctx.set_sender(ADDRESS_1);

//         // and parameter.
//         let transfer = Transfer {
//             from: ADDRESS_0,
//             to: Receiver::from_account(ACCOUNT_1),
//             token_id: token_0(),
//             amount: 1,
//             data: AdditionalData::empty(),
//         };
//         let parameter = TransferParams::from(vec![transfer]);
//         let parameter_bytes = to_bytes(&parameter);
//         ctx.set_parameter(&parameter_bytes);

//         let mut logger = LogRecorder::init();
//         let mut state = initial_state();
//         state.add_operator(&ADDRESS_0, &ADDRESS_1);

//         // Call the contract function.
//         let result: ContractResult<ActionsTree> = contract_transfer(&ctx, &mut logger, &mut state);

//         // Check the result.
//         let actions: ActionsTree = result.expect_report("Results in rejection");
//         claim_eq!(
//             actions,
//             ActionsTree::accept(),
//             "No action should be produced."
//         );

//         // Check the state.
//         let balance0 = state
//             .balance(&token_0(), &ADDRESS_0)
//             .expect_report("Token is expected to exist");
//         let balance1 = state
//             .balance(&token_0(), &ADDRESS_1)
//             .expect_report("Token is expected to exist");
//         claim_eq!(
//             balance0,
//             0,
//             "Token owner balance should be decreased by the transferred amount"
//         );
//         claim_eq!(
//             balance1,
//             1,
//             "Token receiver balance should be increased by the transferred amount"
//         );

//         // Check the logs.
//         claim_eq!(logger.logs.len(), 1, "Only one event should be logged");
//         claim_eq!(
//             logger.logs[0],
//             to_bytes(&Cis1Event::Transfer(TransferEvent {
//                 from: ADDRESS_0,
//                 to: ADDRESS_1,
//                 token_id: token_0(),
//                 amount: 1,
//             })),
//             "Incorrect event emitted"
//         )
//     }

//     /// Test adding an operator succeeds and the appropriate event is logged.
//     #[concordium_test]
//     fn test_add_operator() {
//         // Setup the context
//         let mut ctx = ReceiveContextTest::empty();
//         ctx.set_sender(ADDRESS_0);
//         ctx.set_invoker(ACCOUNT_0);
//         ctx.set_invoker(ACCOUNT_0);

//         // and parameter.
//         let update = UpdateOperator {
//             update: OperatorUpdate::Add,
//             operator: ADDRESS_1,
//         };
//         let parameter = UpdateOperatorParams(vec![update]);
//         let parameter_bytes = to_bytes(&parameter);
//         ctx.set_parameter(&parameter_bytes);

//         let mut logger = LogRecorder::init();
//         let mut state = initial_state();

//         // Call the contract function.
//         let result: ContractResult<ActionsTree> =
//             contract_update_operator(&ctx, &mut logger, &mut state);

//         // Check the result.
//         let actions: ActionsTree = result.expect_report("Results in rejection");
//         claim_eq!(
//             actions,
//             ActionsTree::accept(),
//             "No action should be produced."
//         );

//         // Check the state.
//         let is_operator = state.is_operator(&ADDRESS_1, &ADDRESS_0);
//         claim!(is_operator, "Account should be an operator");

//         // Check the logs.
//         claim_eq!(logger.logs.len(), 1, "One event should be logged");
//         claim_eq!(
//             logger.logs[0],
//             to_bytes(&Cis1Event::<ContractTokenId>::UpdateOperator(
//                 UpdateOperatorEvent {
//                     owner: ADDRESS_0,
//                     operator: ADDRESS_1,
//                     update: OperatorUpdate::Add,
//                 }
//             )),
//             "Incorrect event emitted"
//         )
//     }

//     // Testing burn functionality
//     #[concordium_test]
//     fn test_burn() {
//         // Setup the context
//         let mut ctx = ReceiveContextTest::empty();
//         ctx.set_sender(ADDRESS_0);
//         ctx.set_owner(ACCOUNT_0);
//         ctx.set_invoker(ACCOUNT_0);

//         // and parameter.
//         let mint_data = new_mint_params(ADDRESS_0, token_0());

//         let parameter_bytes = to_bytes(&mint_data);
//         ctx.set_parameter(&parameter_bytes);

//         let mut logger = LogRecorder::init();
//         let mut state = State::new(STORAGE_ADDRESS);

//         // Call the contract function.
//         let _: ContractResult<ActionsTree> = contract_mint(&ctx, &mut logger, &mut state);

//         let parameter_bytes = to_bytes(&token_0());
//         ctx.set_parameter(&parameter_bytes);

//         // Call the contract function.
//         let result: ContractResult<ActionsTree> = contract_burn(&ctx, &mut logger, &mut state);

//         // Check the result
//         let actions = result.expect_report("Results in rejection");
//         claim_eq!(
//             actions,
//             ActionsTree::accept(),
//             "No action should be produced."
//         );

//         // Check the state
//         claim_eq!(
//             state.all_tokens.len(),
//             0,
//             "Expected no tokens in the state."
//         );

//         // Check the logs
//         claim!(
//             logger.logs.contains(&to_bytes(&Cis1Event::Burn(BurnEvent {
//                 token_id: token_0(),
//                 amount: 0,
//                 owner: ADDRESS_0
//             }))),
//             "Expected an event for buning by address ADDRESS_0"
//         );
//     }

//     // Testing update_price functionality
//     #[concordium_test]
//     fn test_update_price() {
//         // Setup the context
//         let mut ctx = ReceiveContextTest::empty();
//         ctx.set_sender(ADDRESS_0);
//         ctx.set_owner(ACCOUNT_0);
//         ctx.set_invoker(ACCOUNT_0);

//         // and parameter.
//         let mint_data = new_mint_params(ADDRESS_0, token_0());

//         let parameter_bytes = to_bytes(&mint_data);
//         ctx.set_parameter(&parameter_bytes);

//         let mut logger = LogRecorder::init();
//         let mut state = State::new(STORAGE_ADDRESS);

//         // Call the contract function.
//         let _: ContractResult<ActionsTree> = contract_mint(&ctx, &mut logger, &mut state);

//         let update_price_params = UpdatePriceParameter {
//             token_id: token_0(),
//             price: Amount::from_ccd(100),
//         };
//         let parameter_bytes = to_bytes(&update_price_params);
//         ctx.set_parameter(&parameter_bytes);

//         // Call the contract function.
//         let result: ContractResult<ActionsTree> =
//             contract_update_price(&ctx, &mut logger, &mut state);

//         // Check the result
//         let actions = result.expect_report("Results in rejection");
//         claim_eq!(
//             actions,
//             ActionsTree::accept(),
//             "No action should be produced."
//         );

//         // Check the logs
//         claim!(
//             logger
//                 .logs
//                 .contains(&to_bytes(&CustomEvent::UpdatePrice(UpdatePriceEvent {
//                     token_id: token_0(),
//                     owner: ADDRESS_0,
//                     from: 0,
//                     to: 100_000_000
//                 }))),
//             "Expected an event for buning by address ADDRESS_0"
//         );
//     }
// }
