use super::*;

/// Init function that creates a new auction
#[init(contract = "BictoryAuction", parameter = "InitParameter")]
fn auction_init(ctx: &impl HasInitContext) -> InitResult<State> {
    let params: InitParameter = ctx.parameter_cursor().get()?;
    Ok(fresh_state(
        params.token_id,
        params.expiry,
        params.storage_address,
    ))
}

/// Authorize the auction contract address as the operator of NFT contract's token owner
#[receive(contract = "BictoryAuction", name = "authorize")]
fn auction_authorize<A: HasActions>(
    ctx: &impl HasReceiveContext,
    state: &mut State,
) -> Result<A, ContractError> {
    let owner = ctx.owner();
    let sender = ctx.sender();
    let storage = StorageContract::new(&state.storage_address);

    // Ensuring sender is the contract owner
    ensure!(sender.matches_account(&owner), ContractError::Unauthorized);

    // Ensuring state is not already authorised
    ensure!(
        state.leaf_contract_address.is_none(),
        CustomContractError::AlreadyAuthorized.into()
    );

    // Adding this contract as operator to the receiving contract
    let action = storage.send_find(
        &ctx.self_address(),
        "BictoryAuction.updateOperator",
        &Bytes(state.token_id.0.clone()),
    );

    Ok(action)
}

/// Receive function in which accounts can bid before the auction end time
#[receive(contract = "BictoryAuction", name = "bid", enable_logger, payable)]
fn auction_bid<A: HasActions>(
    ctx: &impl HasReceiveContext,
    amount: Amount,
    logger: &mut impl HasLogger,
    state: &mut State,
) -> Result<A, ContractError> {
    // Ensuring contract is authorised to perform transaction on NFT contract
    ensure!(
        state.leaf_contract_address.is_some(),
        ContractError::Unauthorized
    );

    match state.auction_state {
        AuctionState::NotSoldYet => {
            let slot_time = ctx.metadata().slot_time();
            ensure!(
                slot_time <= state.expiry,
                CustomContractError::AuctionFinished.into()
            );

            let sender_address = match ctx.sender() {
                Address::Contract(_) => bail!(CustomContractError::OnlyAccountAddress.into()),
                Address::Account(account_address) => account_address,
            };
            let bid_to_update = state
                .bids
                .entry(sender_address)
                .or_insert_with(Amount::zero);

            *bid_to_update += amount;

            // Ensure that the new bid exceeds the highest bid so far
            ensure!(
                *bid_to_update > state.highest_bid,
                CustomContractError::BidTooLow.into()
            );
            state.highest_bid = *bid_to_update;

            // Event for Biding.
            logger.log(&CustomEvent::Biding(BidingEvent {
                token_id: state.token_id.clone(),
                bid: amount,
            }))?;
        }
        AuctionState::Sold(_) => bail!(CustomContractError::AuctionFinalized.into()),
        AuctionState::Canceled => bail!(CustomContractError::AuctionCanceled.into()),
    }

    Ok(A::accept())
}

/// Receive function used to finalize the auction, returning all bids to their
/// senders, except for the winning bid
#[receive(contract = "BictoryAuction", name = "finalize", enable_logger)]
fn auction_finalize<A: HasActions>(
    ctx: &impl HasReceiveContext,
    logger: &mut impl HasLogger,
    state: &mut State,
) -> Result<A, ContractError> {
    let owner = ctx.owner();
    let sender = ctx.sender();
    let storage = StorageContract::new(&state.storage_address);

    // Ensuring sender is the contract owner
    ensure!(sender.matches_account(&owner), ContractError::Unauthorized);

    match state.auction_state {
        AuctionState::NotSoldYet => {
            let slot_time = ctx.metadata().slot_time();
            ensure!(
                slot_time > state.expiry,
                CustomContractError::AuctionStillActive.into()
            );

            let owner = ctx.owner();
            let balance = ctx.self_balance();

            // Event for Finalize.
            logger.log(&CustomEvent::Finalize(state.token_id.clone()))?;

            if balance == Amount::zero() {
                Ok(A::accept())
            } else {
                let mut return_action = A::simple_transfer(&owner, state.highest_bid);
                let mut remaining_bid = None;

                // Return bids that are smaller than highest
                for (addr, &amnt) in state.bids.iter() {
                    if amnt < state.highest_bid {
                        return_action = return_action.and_then(A::simple_transfer(addr, amnt));
                    } else {
                        ensure!(
                            remaining_bid.is_none(),
                            CustomContractError::BidMapError.into()
                        );

                        // Transfer token to highest bidder
                        return_action = return_action.and_then(storage.send_set(
                            <&ByteSlice>::from(state.token_id.0.as_slice()),
                            &[
                                StorageEntryRef::new(FOR_SALE, &false),
                                StorageEntryRef::new(OWNER, &sender),
                            ],
                        ));

                        // Removing this contract as operator to the receiving contract
                        let update_operator: UpdateOperatorParams =
                            UpdateOperatorParams(vec![UpdateOperator {
                                update: OperatorUpdate::Remove,
                                operator: Address::Contract(ctx.self_address()),
                            }]);
                        let receive_name =
                            ReceiveName::new_unchecked("BictoryStorage.updateOperator");
                        return_action = return_action.and_then(send(
                            &state
                                .leaf_contract_address
                                .expect("Leaf contract address cannot be `None`."),
                            receive_name,
                            Amount::zero(),
                            &update_operator,
                        ));

                        state.auction_state = AuctionState::Sold(*addr);
                        remaining_bid = Some((addr, amnt));
                    }
                }

                // Ensure that the only bidder left in the map is the one with the highest bid
                match remaining_bid {
                    Some((_, amount)) => {
                        ensure!(
                            amount == state.highest_bid,
                            CustomContractError::BidMapError.into()
                        );
                        Ok(return_action)
                    }
                    None => bail!(CustomContractError::BidMapError.into()),
                }
            }
        }
        AuctionState::Sold(_) => bail!(CustomContractError::AuctionFinalized.into()),
        AuctionState::Canceled => bail!(CustomContractError::AuctionCanceled.into()),
    }
}

/// Receive function used to cancel the auction, returning all bids to their
/// senders
#[receive(contract = "BictoryAuction", name = "cancel", enable_logger)]
fn auction_cancel<A: HasActions>(
    ctx: &impl HasReceiveContext,
    logger: &mut impl HasLogger,
    state: &mut State,
) -> Result<A, ContractError> {
    let owner = ctx.owner();
    let sender = ctx.sender();

    // Ensuring sender is the contract owner
    ensure!(sender.matches_account(&owner), ContractError::Unauthorized);

    // Ensuring contract is authorised to perform transaction on NFT contract
    ensure!(
        state.leaf_contract_address.is_some(),
        ContractError::Unauthorized
    );

    match state.auction_state {
        AuctionState::NotSoldYet => {
            let balance = ctx.self_balance();

            // Event for Cancel.
            logger.log(&CustomEvent::Cancel(state.token_id.clone()))?;

            // Update auction state
            state.auction_state = AuctionState::Canceled;

            if balance == Amount::zero() {
                Ok(A::accept())
            } else {
                let mut transfer = A::accept();

                // Removing this contract as operator to the receiving contract
                let update_operator: UpdateOperatorParams =
                    UpdateOperatorParams(vec![UpdateOperator {
                        update: OperatorUpdate::Remove,
                        operator: Address::Contract(ctx.self_address()),
                    }]);
                let receive_name = ReceiveName::new_unchecked("BictoryStorage.updateOperator");
                transfer = transfer.and_then(send(
                    &state
                        .leaf_contract_address
                        .expect("Leaf contract address cannot be `None`."),
                    receive_name,
                    Amount::zero(),
                    &update_operator,
                ));

                // Return bids
                for (addr, &amnt) in state.bids.iter() {
                    transfer = transfer.and_then(A::simple_transfer(addr, amnt));
                }

                Ok(transfer)
            }
        }
        AuctionState::Sold(_) => bail!(CustomContractError::AuctionFinalized.into()),
        AuctionState::Canceled => bail!(CustomContractError::AuctionCanceled.into()),
    }
}

#[receive(
    contract = "BictoryAuction",
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

#[concordium_cfg_test]
mod tests {
    use super::*;
    use test_infrastructure::*;

    // A counter for generating new account addresses
    const AUCTION_END: u64 = 1;
    const ACCOUNT_0: AccountAddress = AccountAddress([0u8; 32]);
    const ACCOUNT_1: AccountAddress = AccountAddress([1u8; 32]);
    const ACCOUNT_2: AccountAddress = AccountAddress([2u8; 32]);
    const STORAGE: ContractAddress = ContractAddress {
        index: 1,
        subindex: 1,
    };

    fn token_0() -> ContractTokenId {
        concordium_cis1::TokenIdVec(vec![0, 1])
    }

    fn dummy_fresh_state() -> State {
        dummy_active_state(Amount::zero(), BTreeMap::new())
    }

    fn dummy_active_state(highest: Amount, bids: BTreeMap<AccountAddress, Amount>) -> State {
        State {
            auction_state: AuctionState::NotSoldYet,
            highest_bid: highest,
            token_id: token_0(),
            expiry: Timestamp::from_timestamp_millis(AUCTION_END),
            bids,
            storage_address: STORAGE,
            leaf_contract_address: None,
        }
    }

    fn expect_error<E, T>(expr: Result<T, E>, err: E, msg: &str)
    where
        E: Eq + Debug,
        T: Debug,
    {
        let actual = expr.expect_err(msg);
        assert_eq!(actual, err);
    }

    fn item_expiry_parameter() -> InitParameter {
        InitParameter {
            token_id: token_0(),
            expiry: Timestamp::from_timestamp_millis(AUCTION_END),
            storage_address: STORAGE,
        }
    }

    fn create_parameter_bytes(parameter: &InitParameter) -> Vec<u8> {
        to_bytes(parameter)
    }

    fn parametrized_init_ctx<'a>(parameter_bytes: &'a Vec<u8>) -> InitContextTest<'a> {
        let mut ctx = InitContextTest::empty();
        ctx.set_parameter(parameter_bytes);
        ctx
    }

    fn new_account_ctx<'a>() -> (AccountAddress, ReceiveContextTest<'a>) {
        let account = ACCOUNT_0;
        let ctx = new_ctx(account, account, AUCTION_END);
        (account, ctx)
    }

    fn new_ctx<'a>(
        owner: AccountAddress,
        sender: AccountAddress,
        slot_time: u64,
    ) -> ReceiveContextTest<'a> {
        let mut ctx = ReceiveContextTest::empty();
        ctx.set_sender(Address::Account(sender));
        ctx.set_owner(owner);
        ctx.set_metadata_slot_time(Timestamp::from_timestamp_millis(slot_time));
        ctx
    }

    #[concordium_test]
    /// Test that the smart-contract initialization sets the state correctly
    /// (no bids, active state, indicated auction-end time and item name).
    fn test_init() {
        let parameter_bytes = create_parameter_bytes(&item_expiry_parameter());
        let ctx = parametrized_init_ctx(&parameter_bytes);

        let state_result = auction_init(&ctx);
        let state = state_result.expect("Contract initialization results in error");
        assert_eq!(
            state,
            dummy_fresh_state(),
            "Auction state should be new after initialization"
        );
    }

    #[concordium_test]
    /// Test a sequence of bids and finalizations:
    /// 0. Auction is initialized.
    /// 1. Alice successfully bids 0.1 CCD.
    /// 2. Alice successfully bids another 0.1 CCD, highest bid becomes 0.2 CCD
    /// (the sum of her two bids). 3. Bob successfully bids 0.3 CCD, highest
    /// bid becomes 0.3 CCD. 4. Someone tries to finalize the auction before
    /// its end time. Attempt fails. 5. Dave successfully finalizes the
    /// auction after its end time.    Alice gets her money back, while
    /// Carol (the owner of the contract) collects the highest bid amount.
    /// 6. Attempts to subsequently bid or finalize fail.
    fn test_auction_bid_and_finalize() {
        let parameter_bytes = create_parameter_bytes(&item_expiry_parameter());
        let ctx0 = parametrized_init_ctx(&parameter_bytes);

        let amount = Amount::from_micro_ccd(100);
        let winning_amount = Amount::from_micro_ccd(300);
        let big_amount = Amount::from_micro_ccd(500);

        let mut bid_map = BTreeMap::new();

        // initializing auction
        let mut state = auction_init(&ctx0).expect("Initialization should pass");

        // 1st bid: account1 bids amount1
        let (alice, alice_ctx) = new_account_ctx();
        let mut logger = LogRecorder::init();

        verify_bid(
            &mut state,
            alice,
            &alice_ctx,
            amount,
            &mut bid_map,
            amount,
            &mut logger,
        );

        // 2nd bid: account1 bids `amount` again
        // should work even though it's the same amount because account1 simply
        // increases their bid
        verify_bid(
            &mut state,
            alice,
            &alice_ctx,
            amount,
            &mut bid_map,
            amount + amount,
            &mut logger,
        );

        // 3rd bid: second account
        let (bob, bob_ctx) = new_account_ctx();
        verify_bid(
            &mut state,
            bob,
            &bob_ctx,
            winning_amount,
            &mut bid_map,
            winning_amount,
            &mut logger,
        );

        // trying to finalize auction that is still active
        // (specifically, the bid is submitted at the last moment, at the AUCTION_END
        // time)
        let mut ctx4 = ReceiveContextTest::empty();
        ctx4.set_metadata_slot_time(Timestamp::from_timestamp_millis(AUCTION_END));
        let finres: Result<ActionsTree, _> = auction_finalize(&ctx4, &mut logger, &mut state);
        expect_error(
            finres,
            CustomContractError::AuctionStillActive.into(),
            "Finalizing auction should fail when it's before auction-end time",
        );

        // finalizing auction
        let carol = ACCOUNT_1;
        let dave = ACCOUNT_2;
        let mut ctx5 = new_ctx(carol, dave, AUCTION_END + 1);
        ctx5.set_self_balance(winning_amount);
        let finres2: Result<ActionsTree, _> = auction_finalize(&ctx5, &mut logger, &mut state);
        let _ = finres2.expect("Finalizing auction should work");

        // attempting to finalize auction again should fail
        let finres3: Result<ActionsTree, _> = auction_finalize(&ctx5, &mut logger, &mut state);
        expect_error(
            finres3,
            CustomContractError::AuctionFinalized.into(),
            "Finalizing auction a second time should fail",
        );

        // attempting to bid again should fail
        let res4: Result<ActionsTree, _> =
            auction_bid(&bob_ctx, big_amount, &mut logger, &mut state);
        expect_error(
            res4,
            CustomContractError::AuctionFinalized.into(),
            "Bidding should fail because the auction is finalized",
        );
    }

    fn verify_bid(
        mut state: &mut State,
        _account: AccountAddress,
        ctx: &ContextTest<ReceiveOnlyDataTest>,
        amount: Amount,
        _bid_map: &mut BTreeMap<AccountAddress, Amount>,
        _highest_bid: Amount,
        logger: &mut LogRecorder,
    ) {
        let res: Result<ActionsTree, _> = auction_bid(ctx, amount, logger, &mut state);
        res.expect("Bidding should pass");
    }

    #[concordium_test]
    /// Bids for amounts lower or equal to the highest bid should be rejected.
    fn test_auction_bid_repeated_bid() {
        let (account1, ctx1) = new_account_ctx();

        let parameter_bytes = create_parameter_bytes(&item_expiry_parameter());
        let ctx0 = parametrized_init_ctx(&parameter_bytes);

        let amount = Amount::from_micro_ccd(100);

        let mut bid_map = BTreeMap::new();

        // initializing auction
        let mut state = auction_init(&ctx0).expect("Init results in error");

        let mut logger = LogRecorder::init();

        // 1st bid: account1 bids amount1
        verify_bid(
            &mut state,
            account1,
            &ctx1,
            amount,
            &mut bid_map,
            amount,
            &mut logger,
        );
    }

    #[concordium_test]
    /// Bids for 0 CCD should be rejected.
    fn test_auction_bid_zero() {
        let ctx1 = new_account_ctx().1;
        let parameter_bytes = create_parameter_bytes(&item_expiry_parameter());
        let ctx = parametrized_init_ctx(&parameter_bytes);

        let mut state = auction_init(&ctx).expect("Init results in error");
        let mut logger = LogRecorder::init();

        let res: Result<ActionsTree, _> =
            auction_bid(&ctx1, Amount::zero(), &mut logger, &mut state);
        expect_error(
            res,
            CustomContractError::BidTooLow.into(), /* { bid: Amount::zero(), highest_bid: Amount::zero()} */
            "Bidding zero should fail",
        );
    }

    #[concordium_test]
    fn test_auction_cancel() {
        let parameter_bytes = create_parameter_bytes(&item_expiry_parameter());
        let ctx0 = parametrized_init_ctx(&parameter_bytes);

        let amount = Amount::from_micro_ccd(100);
        let winning_amount = Amount::from_micro_ccd(300);

        let mut bid_map = BTreeMap::new();

        // initializing auction
        let mut state = auction_init(&ctx0).expect("Initialization should pass");

        // 1st bid: account1 bids amount1
        let (alice, mut alice_ctx) = new_account_ctx();
        alice_ctx.set_self_balance(winning_amount);
        let mut logger = LogRecorder::init();

        verify_bid(
            &mut state,
            alice,
            &alice_ctx,
            amount,
            &mut bid_map,
            amount,
            &mut logger,
        );

        let _: Result<ActionsTree, _> = auction_cancel(&alice_ctx, &mut logger, &mut state);
    }
}
