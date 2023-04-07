use commons_v1::*;
use concordium_cis1::*;
use concordium_std::*;

use crate::events::CustomEvent;
use crate::external::*;
use crate::state::State;
use crate::storage;

/// Initialize contract instance with no token types initially.
#[init(contract = "BictoryCnsNft", parameter = "InitParams")]
fn init<S: HasStateApi>(
    ctx: &impl HasInitContext,
    state_builder: &mut StateBuilder<S>,
) -> InitResult<State<S>> {
    let params = InitParams::deserial(&mut ctx.parameter_cursor())?;

    // Construct the initial contract state.
    let state = State::new(state_builder, params, ctx.init_origin());
    Ok(state)
}

/// Function to mint contract with admin, owner and expiry datetime.
///
/// It rejects if:
/// - Fails to parse parameter;
/// - Fails to log `Mint` event;
/// - Sender is not an authorized CNS contract.
#[receive(
    mutable,
    contract = "BictoryCnsNft",
    name = "mint",
    parameter = "CnsMintParams",
    enable_logger
)]
fn mint<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
    logger: &mut impl HasLogger,
) -> ContractResult<()> {
    let params = CnsMintParams::deserial(&mut ctx.parameter_cursor())?;
    let state = host.state();

    ensure!(
        state.is_authorized_cns_contract(&ctx.sender()),
        ContractError::Unauthorized
    );

    let expiry = ctx
        .metadata()
        .slot_time()
        .checked_add(params.duration)
        .ok_or(CustomContractError::InvalidDuration)?;

    let token_storage = state.token_storage;
    storage::insert_token(
        host,
        &token_storage,
        &params.token_id,
        storage::TokenData {
            domain: params.domain,
            owner: params.owner,
            expiry,
            grace: state.grace_on_mint,
            royalty: state.royalty_on_mint,
        },
    )?;

    // Event for minted NFT.
    logger.log(&Cis1Event::Mint(MintEvent {
        token_id: params.token_id,
        amount: 1,
        owner: params.owner,
    }))?;

    Ok(())
}

/// Function to get domain expiry datetime.
///
/// It rejects if:
/// - TODO
#[receive(
    contract = "BictoryCnsNft",
    name = "getTokenExpiry",
    parameter = "TokenParams",
    return_value = "Option<TokenSubscriptionStatus>"
)]
fn get_token_expiry<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<Option<TokenSubscriptionStatus>> {
    let params = TokenParams::deserial(&mut ctx.parameter_cursor())?;
    let slot_time = ctx.metadata().slot_time();
    let state = host.state();

    let status =
        storage::get_token_subscription_data(host, &state.token_storage, &params.token_id)?
            .map(|data| data.into_status(slot_time));

    Ok(status)
}

/// View token data owned by particular address by token_id.
#[receive(
    contract = "BictoryCnsNft",
    name = "getTokenInfo",
    parameter = "TokenParams",
    return_value = "Option<TokenInfo>"
)]
fn get_token_info<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<Option<TokenInfo>> {
    // Parse the parameter.
    let params = TokenParams::deserial(&mut ctx.parameter_cursor())?;
    let state = host.state();

    storage::get_token_info(host, &state.token_storage, &params.token_id)
}

/// Function to get lend expiry datetime.
///
/// It rejects if:
/// - The sender is not the authorized CNS contract.
/// - Fails to parse parameter.
/// - Fails to log Lend event
#[receive(
    contract = "BictoryCnsNft",
    name = "lend",
    parameter = "LendParams",
    enable_logger,
    mutable
)]
fn lend<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
    logger: &mut impl HasLogger,
) -> ContractResult<()> {
    let params = LendParams::deserial(&mut ctx.parameter_cursor())?;
    let slot_time = ctx.metadata().slot_time();
    let state = host.state();

    // Only CNS contract is allowed to lend a token
    ensure!(
        state.is_authorized_cns_contract(&ctx.sender()),
        ContractError::Unauthorized
    );

    let token_storage = state.token_storage;
    let expiry = storage::get_expiry(host, &token_storage, &params.token_id)?
        .ok_or(ContractError::InvalidTokenId)?;

    // It is the responsibility of CNS contract to decide whether to allow extending after expiry or not.
    // Note that lend after token was burnt will return error anyway, because data is wiped in this case.
    let new_expiry = slot_time
        .max(expiry)
        .checked_add(params.extension)
        .ok_or(CustomContractError::InvalidDuration)?;

    storage::update_expiry(host, &token_storage, &params.token_id, new_expiry)?;

    // Event for lend.
    logger.log(&CustomEvent::Lend {
        token: params.token_id,
        expiry: new_expiry,
    })?;

    Ok(())
}

/// Execute a list of domain transfers, in the order of the list.
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
    contract = "BictoryCnsNft",
    name = "transfer",
    parameter = "TransferParameter",
    enable_logger,
    mutable
)]
fn transfer<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
    logger: &mut impl HasLogger,
) -> ContractResult<()> {
    let TransferParams(transfers) = TransferParameter::deserial(&mut ctx.parameter_cursor())?;
    let sender = ctx.sender();
    let slot_time = ctx.metadata().slot_time();

    for transfer in transfers {
        let state = host.state();

        // Authenticate the sender for this transfer
        ensure!(
            state.is_operator(&sender, &transfer.from),
            ContractError::Unauthorized
        );

        let token_storage = state.token_storage;
        // Check owner from storage
        let subscription_data =
            storage::get_token_subscription_data(host, &token_storage, &transfer.token_id)?
                .ok_or_else(|| ContractError::InvalidTokenId)?;
        ensure_eq!(
            subscription_data.owner,
            transfer.from,
            ContractError::InsufficientFunds
        );

        let to_address = transfer.to.address();

        // Check the transfer amount
        match transfer.amount {
            0 => continue,
            1 => (),
            _ => return Err(ContractError::InsufficientFunds),
        }

        // Check token expiry. Return Unauthorized error in grace period and InsufficientFunds
        // error after token expiry
        if subscription_data
            .expiry
            .checked_add(subscription_data.grace)
            .unwrap()
            < slot_time
        {
            return Err(ContractError::InsufficientFunds);
        } else if subscription_data.expiry < slot_time {
            return Err(ContractError::Unauthorized);
        }

        // Update token data
        storage::update_owner(
            host,
            &token_storage,
            &transfer.token_id,
            transfer.to.address(),
        )?;

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
                contract_name: OwnedContractName::new_unchecked(String::from("init_BictoryCnsNft")),
                data: transfer.data,
            };

            host.invoke_contract(
                &address,
                &parameter,
                function.as_receive_name().entrypoint_name(),
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
    contract = "BictoryCnsNft",
    name = "updateOperator",
    parameter = "UpdateOperatorParams",
    enable_logger,
    mutable
)]
fn update_operator<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
    logger: &mut impl HasLogger,
) -> ContractResult<()> {
    let UpdateOperatorParams(params) = UpdateOperatorParams::deserial(&mut ctx.parameter_cursor())?;
    // Get the sender who called this contract function.
    let sender = Address::Account(ctx.invoker());
    let (state, state_builder) = host.state_and_builder();

    for param in params {
        // Update the operator in the state.
        match param.update {
            OperatorUpdate::Add => state.add_operator(&sender, &param.operator, state_builder),
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
    contract = "BictoryCnsNft",
    name = "operatorOf",
    parameter = "OperatorOfQueryParams",
    mutable
)]
fn operator_of<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<()> {
    let params = OperatorOfQueryParams::deserial(&mut ctx.parameter_cursor())?;
    // Build the response.
    let mut response = Vec::with_capacity(params.queries.len());
    let state = host.state();

    for query in params.queries {
        // Query the state for address being an operator of owner.
        let is_operator = state.is_operator(&query.owner, &query.address);
        response.push((query, is_operator));
    }

    // Send back the response.
    host.invoke_contract(
        &params.result_contract,
        &OperatorOfQueryResponse::from(response),
        params.result_function.as_receive_name().entrypoint_name(),
        Amount::zero(),
    )?;

    Ok(())
}

/// Get the balance of given token IDs and addresses. It takes a contract
/// address plus contract function to invoke with the result.
///
/// It rejects if:
/// - It fails to parse the parameter.
/// - Any of the queried `token_id` does not exist.
/// - Message sent back with the result rejects.
#[receive(
    mutable,
    contract = "BictoryCnsNft",
    name = "balanceOf",
    parameter = "ContractBalanceOfQueryParams"
)]
fn balance_of<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<()> {
    let params = ContractBalanceOfQueryParams::deserial(&mut ctx.parameter_cursor())?;
    let slot_time = ctx.metadata().slot_time();
    // Build the response.
    let mut response = Vec::with_capacity(params.queries.len());
    let state = host.state();

    for query in params.queries {
        // Get the owner entry from the storage
        let subscription_data =
            storage::get_token_subscription_data(host, &state.token_storage, &query.token_id)?
                .ok_or_else(|| ContractError::InvalidTokenId)?;

        // Return 1 if address is the owner and token did not expire, 0 otherwise.
        let amount = u64::from(
            subscription_data.owner == query.address && slot_time < subscription_data.expiry,
        );

        response.push((query, amount));
    }

    // Send back the response.
    host.invoke_contract(
        &params.result_contract,
        &BalanceOfQueryResponse::from(response),
        params.result_function.as_receive_name().entrypoint_name(),
        Amount::zero(),
    )?;

    Ok(())
}

/// Function to burn token.
///
/// It rejects if:
/// - Fails to log `BurnEvent`.
/// - Current Time is less than expiry + Grace Period
#[receive(
    mutable,
    contract = "BictoryCnsNft",
    name = "burn",
    parameter = "ContractTokenId",
    enable_logger
)]
fn burn<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
    logger: &mut impl HasLogger,
) -> ContractResult<()> {
    let token_id = ContractTokenId::deserial(&mut ctx.parameter_cursor())?;
    let state = host.state();
    let slot_time = ctx.metadata().slot_time();

    let subscription_data =
        storage::get_token_subscription_data(host, &state.token_storage, &token_id)?
            .ok_or_else(|| ContractError::InvalidTokenId)?;

    ensure!(
        subscription_data
            .expiry
            .checked_add(subscription_data.grace)
            .unwrap()
            < slot_time,
        ContractError::Unauthorized
    );

    let token_storage = state.token_storage;
    host.storage_remove_raw(
        &token_storage,
        &StorageKeysRef::all(Bytes(token_id.0.clone()).as_ref()),
    )?;

    // Log Burn event
    logger.log(&Cis1Event::Burn(BurnEvent {
        token_id,
        amount: 1,
        owner: subscription_data.owner,
    }))?;

    Ok(())
}

#[receive(
    contract = "BictoryCnsNft",
    name = "getRoyalties",
    parameter = "GetRoyaltiesParams",
    return_value = "GetRoyaltiesResponse"
)]
fn get_royalties<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<GetRoyaltiesResponse> {
    let state = host.state();
    let params = GetRoyaltiesParams::deserial(&mut ctx.parameter_cursor())?;

    let royalty = storage::get_token_royalty(host, &state.token_storage, &params.token_id)?
        .ok_or(ContractError::InvalidTokenId)?;

    Ok(GetRoyaltiesResponse {
        royalties: vec![Royalty {
            beneficiary: state.beneficiary,
            percentage: royalty,
        }],
    })
}

/// Function to manage addresses that are allowed to maintain and modify the state of the contract.
///
///  It rejects if:
///  - Fails to parse `AuthorityUpdateParams` parameters.
///  - If sender is neither one of the admins nor one of the maintainers.
#[receive(
    mutable,
    contract = "BictoryCnsNft",
    name = "updateAuthority",
    parameter = "AuthorityUpdateParams"
)]
fn update_authority<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ReceiveResult<()> {
    let state = host.state_mut();
    let params = AuthorityUpdateParams::deserial(&mut ctx.parameter_cursor())?;
    let sender = ctx.sender();
    state.authority.handle_update(sender, params)
}

/// Function to view addresses that are allowed to maintain and modify the state of the contract.
#[receive(
    contract = "BictoryCnsNft",
    name = "viewAuthority",
    parameter = "AuthorityViewParams",
    return_value = "Vec<Address>"
)]
fn view_authority<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &impl HasHost<State<S>, StateApiType = S>,
) -> ReceiveResult<Vec<Address>> {
    let params = AuthorityViewParams::deserial(&mut ctx.parameter_cursor())?;
    Ok(host.state().authority.handle_view(params))
}

/// Function to update internal values. This includes:
/// - CnsContract. Address of authorised CNS contract, that is allowed to make changes to NFT.
/// - Royalty. fee percentage for token sale. Gets assigned to a token on mint.
/// - Beneficiary. Account address that receives the fee.
///
///  It rejects if:
///  - Fails to parse `UpdateInternalAddressParams` parameters.
///  - If sender is neither one of the admins nor one of the maintainers.
#[receive(
    mutable,
    contract = "BictoryCnsNft",
    name = "updateInternalValue",
    parameter = "UpdateInternalValueParams"
)]
fn update_internal_value<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<()> {
    if !host.state().authority.has_maintainer_rights(&ctx.sender()) {
        return Err(ContractError::Unauthorized);
    }

    let mut state = host.state_mut();
    let params = UpdateInternalValueParams::deserial(&mut ctx.parameter_cursor())?;

    match params {
        UpdateInternalValueParams::CnsContract(update) => match update.operation {
            UpdateOperation::Add => {
                state.cns_contracts.insert(update.address);
            }
            UpdateOperation::Remove => {
                state.cns_contracts.remove(&update.address);
            }
        },
        UpdateInternalValueParams::Royalty(percentage) => state.royalty_on_mint = percentage,
        UpdateInternalValueParams::Beneficiary(account) => state.beneficiary = account,
    }

    Ok(())
}

#[receive(
    contract = "BictoryCnsNft",
    name = "viewInternalValue",
    parameter = "ViewInternalValueParams",
    return_value = "ViewInternalValueResult"
)]
fn view_internal_value<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<ViewInternalValueResult> {
    let state = host.state();
    let params = ViewInternalValueParams::deserial(&mut ctx.parameter_cursor())?;

    let value = match params {
        ViewInternalValueParams::CnsContract(view) => ViewInternalValueResult::CnsContract(
            state
                .cns_contracts
                .iter()
                .skip(view.skip as usize)
                .take(view.show as usize)
                .map(|contract| *contract)
                .collect(),
        ),
        ViewInternalValueParams::Royalty => ViewInternalValueResult::Royalty(state.royalty_on_mint),
        ViewInternalValueParams::Beneficiary => {
            ViewInternalValueResult::Beneficiary(state.beneficiary)
        }
    };

    Ok(value)
}

#[concordium_cfg_test]
mod tests {
    use super::*;
    use commons_v1::test::*;
    use test_infrastructure::*;

    const ADMIN: AccountAddress = AccountAddress([1; 32]);
    const MAINTAINER: AccountAddress = AccountAddress([2; 32]);

    const USER_1: AccountAddress = AccountAddress([16; 32]);
    const USER_2: AccountAddress = AccountAddress([17; 32]);

    const CONTRACT_1: ContractAddress = ContractAddress {
        index: 16,
        subindex: 16,
    };

    const CNS: ContractAddress = ContractAddress {
        index: 5,
        subindex: 0,
    };

    const STORAGE: ContractAddress = ContractAddress {
        index: 1,
        subindex: 0,
    };

    fn slot_time() -> Timestamp {
        Timestamp::from_timestamp_millis(0)
            .checked_add(Duration::from_days(1000))
            .unwrap()
    }

    fn token_0() -> ContractTokenId {
        TokenIdVec([32; 32].into())
    }

    fn storage_ownership_data(
        owner: &Address,
        expiry: &Timestamp,
        grace: &Duration,
    ) -> StorageGetEntryResult {
        StorageGetEntryResult {
            prefix: Bytes(token_0().0),
            entries: vec![
                MaybeStorageEntry {
                    key: Bytes("owner".as_bytes().into()),
                    value: Some(Bytes(to_bytes(owner))),
                },
                MaybeStorageEntry {
                    key: Bytes("expiry".as_bytes().into()),
                    value: Some(Bytes(to_bytes(expiry))),
                },
                MaybeStorageEntry {
                    key: Bytes("grace".as_bytes().into()),
                    value: Some(Bytes(to_bytes(grace))),
                },
            ],
        }
    }

    /// Test helper function which creates a contract state with two tokens with
    /// id `token_0` and id `token_1` owned by `ADDRESS_0`
    fn default_host() -> TestHost<State<TestStateApi>> {
        let mut ctx = TestInitContext::empty();
        let params = InitParams {
            storage_contract: STORAGE,
            royalty_on_mint: Percentage::from_percent(3),
            grace_on_mint: Duration::from_days(60),
            beneficiary: ADMIN,
        };
        let bytes = to_bytes(&params);
        // admin is initialized to `ctx.origin()`
        ctx.set_init_origin(ADMIN).set_parameter(&bytes);
        let mut state_builder = TestStateBuilder::new();

        // Call the init method.
        let state =
            init(&ctx, &mut state_builder).expect_report("Failed during init_BictoryCnsNft");

        let mut host = TestHost::new(state, state_builder);

        let mut ctx = TestReceiveContext::empty();
        let params = AuthorityUpdateParams {
            field: commons_v1::AuthorityField::Maintainer,
            kind: commons_v1::AuthorityUpdateKind::Add,
            address: Address::Account(MAINTAINER),
        };
        let bytes = to_bytes(&params);
        ctx.set_sender(Address::Account(ADMIN))
            .set_parameter(&bytes);
        let result = update_authority(&ctx, &mut host);
        claim_eq!(result, Ok(()));

        let mut ctx = TestReceiveContext::empty();
        let params = UpdateInternalValueParams::CnsContract(AddressUpdate {
            operation: UpdateOperation::Add,
            address: CNS,
        });
        let bytes = to_bytes(&params);
        ctx.set_sender(Address::Account(MAINTAINER))
            .set_parameter(&bytes);
        let result = update_internal_value(&ctx, &mut host);
        claim_eq!(result, Ok(()));

        host
    }

    #[concordium_test]
    fn test_init_test_state() {
        let host = default_host();
        let state = host.state();

        // Assert properties
        claim_eq!(state.token_storage, STORAGE);

        // Admin has full rights
        claim!(state.authority.has_admin_rights(&Address::Account(ADMIN)));
        claim!(state
            .authority
            .has_maintainer_rights(&Address::Account(ADMIN)));

        // Maintainer only has maintainer rights
        claim!(!state
            .authority
            .has_admin_rights(&Address::Account(MAINTAINER)));
        claim!(state
            .authority
            .has_maintainer_rights(&Address::Account(MAINTAINER)));
    }

    #[concordium_test]
    fn test_mint() {
        let mut host = default_host();

        let mut ctx = TestReceiveContext::empty();
        let params = CnsMintParams {
            token_id: token_0(),
            domain: From::from("test.ccd"),
            owner: Address::Account(USER_1),
            duration: Duration::from_hours(24 * 365 + 6),
        };
        let bytes = to_bytes(&params);
        ctx.set_sender(Address::Contract(CNS))
            .set_parameter(&bytes)
            .set_metadata_slot_time(slot_time());

        host.setup_mock_entrypoint(
            STORAGE,
            OwnedEntrypointName::new_unchecked("insert".into()),
            parse_and_ok_mock::<StorageEntries, _>(()),
        );

        let mut logger = TestLogger::init();

        let result = mint(&ctx, &mut host, &mut logger);

        claim_eq!(result, Ok(()));
    }

    #[concordium_test]
    fn test_get_token_expiry() {
        let mut host = default_host();

        let mut ctx = TestReceiveContext::empty();
        let bytes = to_bytes(&TokenParams {
            token_id: token_0(),
        });
        ctx.set_parameter(&bytes)
            .set_metadata_slot_time(slot_time());

        host.setup_mock_entrypoint(
            STORAGE,
            OwnedEntrypointName::new_unchecked("get".into()),
            parse_and_ok_mock::<StorageKeys, _>(Some(storage_ownership_data(
                &Address::Account(USER_1),
                &slot_time().checked_add(Duration::from_days(100)).unwrap(),
                &Duration::from_days(60),
            ))),
        );

        let result = get_token_expiry(&ctx, &host)
            .expect_report("Unexpected error during 'getTokenExpiry' call");

        claim_eq!(
            result,
            Some(TokenSubscriptionStatus {
                owner: Address::Account(USER_1),
                expiry: SubscriptionExpiryStatus::Owned(
                    slot_time().checked_add(Duration::from_days(100)).unwrap()
                )
            })
        );
    }

    #[concordium_test]
    fn test_get_token_info() {
        let mut host = default_host();

        let mut ctx = TestReceiveContext::empty();
        let bytes = to_bytes(&TokenParams {
            token_id: token_0(),
        });
        ctx.set_parameter(&bytes)
            .set_metadata_slot_time(slot_time());

        host.setup_mock_entrypoint(
            STORAGE,
            OwnedEntrypointName::new_unchecked("get".into()),
            parse_and_ok_mock::<StorageKeys, _>(Some(StorageGetEntryResult {
                prefix: Bytes(token_0().0),
                entries: vec![
                    MaybeStorageEntry {
                        key: Bytes("domain".as_bytes().into()),
                        value: Some(Bytes(to_bytes(&"test.ccd"))),
                    },
                    MaybeStorageEntry {
                        key: Bytes("royalty".as_bytes().into()),
                        value: Some(Bytes(to_bytes(&Percentage::from_percent(3)))),
                    },
                ],
            })),
        );

        let result = get_token_info(&ctx, &host)
            .expect_report("Unexpected error during 'getTokenInfo' call");

        claim_eq!(
            result,
            Some(TokenInfo {
                domain: String::from("test.ccd"),
                royalty: Percentage::from_percent(3),
            })
        );
    }

    #[concordium_test]
    fn test_lend() {
        let mut host = default_host();

        let mut ctx = TestReceiveContext::empty();
        let params = CnsMintParams {
            token_id: token_0(),
            domain: From::from("test.ccd"),
            owner: Address::Account(USER_1),
            duration: Duration::from_hours(24 * 365 + 6),
        };
        let bytes = to_bytes(&params);
        ctx.set_sender(Address::Contract(CNS))
            .set_parameter(&bytes)
            .set_metadata_slot_time(slot_time());

        host.setup_mock_entrypoint(
            STORAGE,
            OwnedEntrypointName::new_unchecked("get".into()),
            parse_and_ok_mock::<StorageKeys, _>(Some(storage_ownership_data(
                &Address::Account(USER_1),
                &slot_time().checked_add(Duration::from_days(100)).unwrap(),
                &Duration::from_days(60),
            ))),
        );
        host.setup_mock_entrypoint(
            STORAGE,
            OwnedEntrypointName::new_unchecked("update".into()),
            parse_and_ok_mock::<StorageEntries, _>(()),
        );

        let mut logger = TestLogger::init();

        let result = lend(&ctx, &mut host, &mut logger);

        claim_eq!(result, Ok(()));
    }

    #[concordium_test]
    fn test_transfer_to_account() {
        let mut host = default_host();

        let mut ctx = TestReceiveContext::empty();
        let params = TransferParams(vec![Transfer {
            token_id: token_0(),
            amount: 1,
            from: Address::Account(USER_1),
            to: Receiver::Account(USER_2),
            data: AdditionalData::from(vec![]),
        }]);
        let bytes = to_bytes(&params);
        ctx.set_sender(Address::Account(USER_1))
            .set_parameter(&bytes)
            .set_metadata_slot_time(slot_time());

        host.setup_mock_entrypoint(
            STORAGE,
            OwnedEntrypointName::new_unchecked("get".into()),
            parse_and_ok_mock::<StorageKeys, _>(Some(storage_ownership_data(
                &Address::Account(USER_1),
                &slot_time().checked_add(Duration::from_days(100)).unwrap(),
                &Duration::from_days(60),
            ))),
        );
        host.setup_mock_entrypoint(
            STORAGE,
            OwnedEntrypointName::new_unchecked("update".into()),
            parse_and_ok_mock::<StorageEntries, _>(()),
        );

        let mut logger = TestLogger::init();

        let result = transfer(&ctx, &mut host, &mut logger);

        claim_eq!(result, Ok(()));
    }

    #[concordium_test]
    fn test_transfer_to_contract() {
        let mut host = default_host();

        let mut ctx = TestReceiveContext::empty();
        let params = TransferParams(vec![Transfer {
            token_id: token_0(),
            amount: 1,
            from: Address::Account(USER_1),
            to: Receiver::Contract(
                CONTRACT_1,
                OwnedReceiveName::new_unchecked("Listing.list".into()),
            ),
            data: AdditionalData::from(to_bytes(&Amount::from_ccd(100))),
        }]);
        let bytes = to_bytes(&params);
        ctx.set_sender(Address::Account(USER_1))
            .set_parameter(&bytes)
            .set_metadata_slot_time(slot_time());

        host.setup_mock_entrypoint(
            STORAGE,
            OwnedEntrypointName::new_unchecked("get".into()),
            parse_and_ok_mock::<StorageKeys, _>(Some(storage_ownership_data(
                &Address::Account(USER_1),
                &slot_time().checked_add(Duration::from_days(100)).unwrap(),
                &Duration::from_days(60),
            ))),
        );
        host.setup_mock_entrypoint(
            STORAGE,
            OwnedEntrypointName::new_unchecked("update".into()),
            parse_and_ok_mock::<StorageEntries, _>(()),
        );
        host.setup_mock_entrypoint(
            CONTRACT_1,
            OwnedEntrypointName::new_unchecked("list".into()),
            parse_and_check_mock::<OnReceivingCis1Params<TokenIdVec>, _>(
                |params| from_bytes::<Amount>(params.data.as_ref()).is_ok(),
                (),
            ),
        );

        let mut logger = TestLogger::init();

        let result = transfer(&ctx, &mut host, &mut logger);

        claim_eq!(result, Ok(()));
    }
}
