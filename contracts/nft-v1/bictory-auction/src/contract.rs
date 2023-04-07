use super::*;

/// Init function that creates a new auction
#[init(contract = "BictoryAuction", parameter = "InitParameter")]
fn init<S: HasStateApi>(
    ctx: &impl HasInitContext,
    state_builder: &mut StateBuilder<S>,
) -> InitResult<State<S>> {
    let parameter: InitParameter = ctx.parameter_cursor().get()?;
    Ok(State::empty(
        parameter.item,
        parameter.expiry,
        state_builder,
    ))
}

/// Authorize the auction contract address as the operator of NFT contract's token owner
#[receive(contract = "BictoryAuction", name = "authorize", mutable)]
fn authorize<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<()> {
    let owner = ctx.owner();
    let sender = ctx.sender();
    let state = host.state();
    // Ensuring sender is the contract owner
    ensure!(sender.matches_account(&owner), ContractError::Unauthorized);

    // Ensuring state is not already authorised
    ensure!(
        !state.viewable_state.is_authorised,
        CustomContractError::AlreadyAuthorized.into()
    );

    host.state_mut().viewable_state.is_authorised = true;

    // Adding this contract as operator to the receiving contract
    let update_operator: UpdateOperatorParams = UpdateOperatorParams(vec![UpdateOperator {
        update: OperatorUpdate::Add,
        operator: Address::Contract(ctx.self_address()),
    }]);
    let entrypoint_name = EntrypointName::new_unchecked("updateOperator");
    host.invoke_contract(
        &host.state().viewable_state.item.contract.clone(),
        &update_operator,
        entrypoint_name,
        Amount::zero(),
    )?;

    Ok(())
}

/// Receive function in which accounts can bid before the auction end time
#[receive(
    contract = "BictoryAuction",
    name = "bid",
    mutable,
    enable_logger,
    payable
)]
fn bid<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
    amount: Amount,
    logger: &mut impl HasLogger,
) -> ContractResult<()> {
    let state = host.state_mut();
    match state.viewable_state.auction_state {
        AuctionState::NotSoldYet => {
            let slot_time = ctx.metadata().slot_time();
            ensure!(
                slot_time <= state.viewable_state.expiry,
                CustomContractError::AuctionFinished.into()
            );

            let sender_address = match ctx.sender() {
                Address::Contract(_) => bail!(CustomContractError::OnlyAccountAddress.into()),
                Address::Account(account_address) => account_address,
            };
            let mut bid_to_update = state
                .bids
                .entry(sender_address)
                .or_insert_with(Amount::zero);

            *bid_to_update += amount;

            // Ensure that the new bid exceeds the highest bid so far
            ensure!(
                *bid_to_update > state.viewable_state.highest_bid,
                CustomContractError::BidTooLow.into()
            );
            state.viewable_state.highest_bid = *bid_to_update;

            // Event for Biding.
            logger.log(&CustomEvent::Biding(BidingEvent {
                account: state.viewable_state.item.clone(),
                bid: amount,
            }))?;
        }
        AuctionState::Sold(_) => bail!(CustomContractError::AuctionFinalized.into()),
        AuctionState::Canceled => bail!(CustomContractError::AuctionCanceled.into()),
    }

    Ok(())
}

/// Receive function used to finalize the auction, returning all bids to their
/// senders, except for the winning bid
#[receive(contract = "BictoryAuction", name = "finalize", mutable, enable_logger)]
fn finalize<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
    logger: &mut impl HasLogger,
) -> ContractResult<()> {
    let state = host.state();
    let contract = state.viewable_state.item.contract;

    // Ensuring contract is authorised to perform transaction on NFT contract
    ensure!(
        state.viewable_state.is_authorised,
        ContractError::Unauthorized
    );

    match state.viewable_state.auction_state {
        AuctionState::NotSoldYet => {
            let slot_time = ctx.metadata().slot_time();
            ensure!(
                slot_time > state.viewable_state.expiry,
                CustomContractError::AuctionStillActive.into()
            );

            let owner = ctx.owner();
            let balance = host.self_balance();

            // Event for Finalize.
            logger.log(&CustomEvent::Finalize(state.viewable_state.item.clone()))?;

            if balance == Amount::zero() {
                Ok(())
            } else {
                host.invoke_transfer(&owner, state.viewable_state.highest_bid)?;
                let mut remaining_bid = None;

                // Return bids that are smaller than highest
                for (addr, amnt) in state.bids.iter() {
                    if *amnt < state.viewable_state.highest_bid {
                        host.invoke_transfer(&addr, *amnt)?;
                    } else {
                        ensure!(
                            remaining_bid.is_none(),
                            CustomContractError::BidMapError.into()
                        );
                        remaining_bid = Some((addr, amnt));
                    }
                }

                // Ensure that the only bidder left in the map is the one with the highest bid
                match remaining_bid {
                    Some((addr, amount)) => {
                        ensure!(
                            amount.eq(&state.viewable_state.highest_bid),
                            CustomContractError::BidMapError.into()
                        );

                        // Transfer token to highest bidder
                        let transfer = Transfer {
                            token_id: state.viewable_state.item.id.clone(),
                            amount: ContractTokenAmount::from(1),
                            from: Address::Account(owner),
                            to: Receiver::Account(*addr),
                            data: AdditionalData::empty(),
                        };
                        let parameter = TransferParams(vec![transfer]);
                        let mut entrypoint_name = EntrypointName::new_unchecked("transfer");

                        host.state_mut().viewable_state.auction_state = AuctionState::Sold(*addr);

                        host.invoke_contract(
                            &contract,
                            &parameter,
                            entrypoint_name,
                            Amount::zero(),
                        )?;

                        // Removing this contract as operator to the receiving contract
                        let update_operator: UpdateOperatorParams =
                            UpdateOperatorParams(vec![UpdateOperator {
                                update: OperatorUpdate::Remove,
                                operator: Address::Contract(ctx.self_address()),
                            }]);
                        entrypoint_name = EntrypointName::new_unchecked("updateOperator");
                        host.invoke_contract(
                            &contract,
                            &update_operator,
                            entrypoint_name,
                            Amount::zero(),
                        )?;

                        Ok(())
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
#[receive(contract = "BictoryAuction", name = "cancel", mutable, enable_logger)]
fn cancel<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
    logger: &mut impl HasLogger,
) -> ContractResult<()> {
    let owner = ctx.owner();
    let sender = ctx.sender();
    let state = host.state();

    // Ensuring sender is the contract owner
    ensure!(sender.matches_account(&owner), ContractError::Unauthorized);

    // Ensuring contract is authorised to perform transaction on NFT contract
    ensure!(
        state.viewable_state.is_authorised,
        ContractError::Unauthorized
    );

    match state.viewable_state.auction_state {
        AuctionState::NotSoldYet => {
            let balance = host.self_balance();

            // Event for Cancel.
            logger.log(&CustomEvent::Cancel(state.viewable_state.item.clone()))?;

            if balance == Amount::zero() {
                Ok(())
            } else {
                // Return bids
                for (addr, amnt) in state.bids.iter() {
                    host.invoke_transfer(&addr, *amnt)?;
                }

                // Removing this contract as operator to the receiving contract
                let update_operator: UpdateOperatorParams =
                    UpdateOperatorParams(vec![UpdateOperator {
                        update: OperatorUpdate::Remove,
                        operator: Address::Contract(ctx.self_address()),
                    }]);
                let entrypoint_name = EntrypointName::new_unchecked("updateOperator");

                // Update auction state
                host.state_mut().viewable_state.auction_state = AuctionState::Canceled;

                host.invoke_contract(
                    &host.state().viewable_state.item.contract.clone(),
                    &update_operator,
                    entrypoint_name,
                    Amount::zero(),
                )?;

                Ok(())
            }
        }
        AuctionState::Sold(_) => bail!(CustomContractError::AuctionFinalized.into()),
        AuctionState::Canceled => bail!(CustomContractError::AuctionCanceled.into()),
    }
}

/// View function that returns the contents of the state except the map of
/// individual bids.
#[receive(
    contract = "BictoryAuction",
    name = "view",
    return_value = "ViewableState"
)]
fn view<S: HasStateApi>(
    _ctx: &impl HasReceiveContext,
    host: &impl HasHost<State<S>, StateApiType = S>,
) -> ReceiveResult<ViewableState> {
    Ok(host.state().viewable_state.clone())
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

    fn token_0() -> ContractTokenId {
        concordium_cis2::TokenIdVec(vec![0, 1])
    }

    fn dummy_fresh_state<S: HasStateApi>(state_builder: &mut StateBuilder<S>) -> State<S> {
        dummy_active_state(Amount::zero(), state_builder)
    }

    fn dummy_token() -> Token {
        Token {
            contract: ContractAddress {
                index: 1,
                subindex: 0,
            },
            id: token_0(),
        }
    }

    fn dummy_active_state<S: HasStateApi>(
        highest: Amount,
        state_builder: &mut StateBuilder<S>,
    ) -> State<S> {
        State {
            viewable_state: ViewableState {
                auction_state: AuctionState::NotSoldYet,
                highest_bid: highest,
                item: dummy_token(),
                expiry: Timestamp::from_timestamp_millis(AUCTION_END),
                is_authorised: false,
            },
            bids: state_builder.new_map(),
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
            item: dummy_token(),
            expiry: Timestamp::from_timestamp_millis(AUCTION_END),
        }
    }

    fn create_parameter_bytes(parameter: &InitParameter) -> Vec<u8> {
        to_bytes(parameter)
    }

    fn parametrized_init_ctx<'a>(parameter_bytes: &'a Vec<u8>) -> TestInitContext<'a> {
        let mut ctx = TestInitContext::empty();
        ctx.set_parameter(parameter_bytes);
        ctx
    }

    fn new_account_ctx<'a>() -> (AccountAddress, TestReceiveContext<'a>) {
        let account = ACCOUNT_0;
        let ctx = new_ctx(account, account, AUCTION_END);
        (account, ctx)
    }

    fn new_ctx<'a>(
        owner: AccountAddress,
        sender: AccountAddress,
        slot_time: u64,
    ) -> TestReceiveContext<'a> {
        let mut ctx = TestReceiveContext::empty();
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
        let mut state_builder = TestStateBuilder::new();

        let state_result = init(&ctx, &mut state_builder);
        let state = state_result.expect("Contract initialization results in error");
        claim_eq!(
            state.viewable_state.auction_state,
            AuctionState::NotSoldYet,
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

        let mut state_builder = TestStateBuilder::new();
        let mut bid_map = state_builder.new_map();

        // initializing auction
        let mut state = init(&ctx0, &mut state_builder).expect("Initialization should pass");
        let mut host = TestHost::new(state, state_builder);

        // 1st bid: account1 bids amount1
        let (alice, alice_ctx) = new_account_ctx();
        let mut logger = TestLogger::init();

        verify_bid(
            &mut host,
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
            &mut host,
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
            &mut host,
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
        let mut ctx4 = TestReceiveContext::empty();
        ctx4.set_metadata_slot_time(Timestamp::from_timestamp_millis(AUCTION_END));
        let finres: ContractResult<()> = finalize(&ctx4, &mut host, &mut logger);
        expect_error(
            finres,
            CustomContractError::AuctionStillActive.into(),
            "Finalizing auction should fail when it's before auction-end time",
        );

        // finalizing auction
        let carol = ACCOUNT_1;
        let dave = ACCOUNT_2;
        let mut ctx5 = new_ctx(carol, dave, AUCTION_END + 1);
        host.set_self_balance(winning_amount);
        let finres2: ContractResult<()> = finalize(&ctx5, &mut host, &mut logger);
        let _ = finres2.expect("Finalizing auction should work");

        // attempting to finalize auction again should fail
        let finres3: ContractResult<()> = finalize(&ctx5, &mut host, &mut logger);
        expect_error(
            finres3,
            CustomContractError::AuctionFinalized.into(),
            "Finalizing auction a second time should fail",
        );

        // attempting to bid again should fail
        let res4: ContractResult<()> = bid(&bob_ctx, &mut host, big_amount, &mut logger);
        expect_error(
            res4,
            CustomContractError::AuctionFinalized.into(),
            "Bidding should fail because the auction is finalized",
        );
    }

    fn verify_bid<S: HasStateApi>(
        mut host: &mut TestHost<State<TestStateApi>>,
        _account: AccountAddress,
        ctx: &TestContext<TestReceiveOnlyData>,
        amount: Amount,
        _bid_map: &mut StateMap<AccountAddress, Amount, S>,
        _highest_bid: Amount,
        logger: &mut TestLogger,
    ) {
        let res: ContractResult<()> = bid(ctx, host, amount, logger);
        res.expect("Bidding should pass");
    }

    #[concordium_test]
    /// Bids for amounts lower or equal to the highest bid should be rejected.
    fn test_auction_bid_repeated_bid() {
        let (account1, ctx1) = new_account_ctx();

        let parameter_bytes = create_parameter_bytes(&item_expiry_parameter());
        let ctx0 = parametrized_init_ctx(&parameter_bytes);

        let amount = Amount::from_micro_ccd(100);

        let mut state_builder = TestStateBuilder::new();
        let mut bid_map = state_builder.new_map();

        // initializing auction
        let mut state = init(&ctx0, &mut state_builder).expect("Init results in error");
        let mut host = TestHost::new(state, state_builder);

        let mut logger = TestLogger::init();

        // 1st bid: account1 bids amount1
        verify_bid(
            &mut host,
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
        let mut state_builder = TestStateBuilder::new();

        let mut state = init(&ctx, &mut state_builder).expect("Init results in error");
        let mut host = TestHost::new(state, state_builder);
        let mut logger = TestLogger::init();

        let res: ContractResult<()> = bid(&ctx1, &mut host, Amount::zero(), &mut logger);
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

        let mut state_builder = TestStateBuilder::new();
        let mut bid_map = state_builder.new_map();

        // initializing auction
        let mut state = init(&ctx0, &mut state_builder).expect("Initialization should pass");
        let mut host = TestHost::new(state, state_builder);

        // 1st bid: account1 bids amount1
        let (alice, mut alice_ctx) = new_account_ctx();
        host.set_self_balance(winning_amount);
        let mut logger = TestLogger::init();

        verify_bid(
            &mut host,
            alice,
            &alice_ctx,
            amount,
            &mut bid_map,
            amount,
            &mut logger,
        );

        let _: ContractResult<()> = cancel(&alice_ctx, &mut host, &mut logger);
    }
}
