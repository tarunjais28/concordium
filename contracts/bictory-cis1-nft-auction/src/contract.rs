use commons_v1::{
    AuthorityUpdateParams, AuthorityViewParams, CustomContractError, Percentage, Royalty, Token,
};
use concordium_cis1::{OnReceivingCis1Params, TokenIdVec};
use concordium_std::*;

use crate::events::*;
use crate::external::*;
use crate::nft;
use crate::state::{AuctionResult, State};

/// Initialize the listing contract with an empty list of lots.
#[init(contract = "BictoryNftAuction", parameter = "InitParams")]
fn contract_init<S: HasStateApi>(
    ctx: &impl HasInitContext,
    state_builder: &mut StateBuilder<S>,
) -> InitResult<State<S>> {
    let params = InitParams::deserial(&mut ctx.parameter_cursor())?;
    Ok(State::new(
        state_builder,
        params.beneficiary,
        params.royalty,
        ctx.init_origin(),
    ))
}

#[receive(
    mutable,
    contract = "BictoryNftAuction",
    name = "auction",
    parameter = "OnReceivingCis1Params<TokenIdVec>",
    enable_logger
)]
fn contract_auction<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
    logger: &mut impl HasLogger,
) -> ReceiveResult<()> {
    let transfer_info = OnReceivingCis1Params::<TokenIdVec>::deserial(&mut ctx.parameter_cursor())?;
    // Do not auction anything if no tokens were transfered
    if transfer_info.amount == 0 {
        return Ok(());
    }
    // Amount of tokens over 1 is not currently supported
    ensure!(
        transfer_info.amount == 1,
        CustomContractError::Unsupported.into()
    );

    let owner = if let Address::Account(owner) = transfer_info.from {
        owner
    } else {
        bail!(CustomContractError::Unsupported.into());
    };

    let contract = if let Address::Contract(sender) = ctx.sender() {
        sender
    } else {
        bail!(CustomContractError::ContractOnly.into());
    };

    let lot_info: LotInfo = from_bytes(transfer_info.data.as_ref())?;

    let royalties = nft::get_royalties(host, &contract, &transfer_info.token_id)?;

    if royalties
        .iter()
        .fold(host.state().royalty, |acc, x| acc + x.percentage)
        > Percentage::from_percent(100)
    {
        bail!(CustomContractError::InvalidRoyalty.into());
    }

    // Finalize event has to be logged during finalization, and royalties is the only non-static part of it. To prevent
    // failing on final auction step, this check is requried to ensure that event size will be under 512 bytes and it
    // will be logged successfuly.
    ensure!(
        royalties.len() < 10,
        CustomContractError::InvalidRoyalty.into()
    );

    logger.log(&AuctionEvents::auction(
        &contract,
        &transfer_info.token_id,
        &owner,
        &lot_info,
    ))?;

    // Create new lot with token
    host.state_mut().auction(
        contract,
        transfer_info.token_id,
        owner,
        lot_info,
        ctx.metadata().slot_time(),
        royalties,
    )?;

    Ok(())
}

#[receive(
    mutable,
    payable,
    contract = "BictoryNftAuction",
    name = "bid",
    parameter = "Token",
    enable_logger
)]
fn contract_bid<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>>,
    amount: Amount,
    logger: &mut impl HasLogger,
) -> ReceiveResult<()> {
    let token = Token::deserial(&mut ctx.parameter_cursor())?;

    let bidder = if let Address::Account(bidder) = ctx.sender() {
        bidder
    } else {
        bail!(CustomContractError::OnlyAccountAddress.into());
    };

    let previous_bid = host
        .state_mut()
        .bid(&token, ctx.metadata().slot_time(), bidder, amount)?;

    logger.log(&AuctionEvents::bid(
        &token.contract,
        &token.id,
        &bidder,
        amount,
    ))?;

    // Refund previous bid
    if let Some(bid) = previous_bid {
        host.invoke_transfer(&bid.account, bid.amount)?;
    }

    Ok(())
}

/// Finalize the auction.
///
/// This function must ensure that token and bids can be refunded.
#[receive(
    mutable,
    contract = "BictoryNftAuction",
    name = "finalize",
    parameter = "Token",
    enable_logger
)]
fn contract_finalize<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>>,
    logger: &mut impl HasLogger,
) -> ReceiveResult<()> {
    let token = Token::deserial(&mut ctx.parameter_cursor())?;

    // Try to finalize auction and get outcome
    let outcome = host
        .state_mut()
        .finalize(&token, ctx.metadata().slot_time())?;

    match outcome {
        AuctionResult::Winner {
            previous_owner,
            winning_bid,
            royalties,
        } => {
            // Try to transfer token to auction winner.
            if nft::transfer(
                host,
                &token,
                &Address::Contract(ctx.self_address()),
                &winning_bid.account,
            )
            .is_err()
            {
                // Log the auction abort event
                logger.log(&AuctionEvents::abort(
                    &token.contract,
                    &token.id,
                    &previous_owner,
                    &winning_bid.account,
                    winning_bid.amount,
                ))?;

                // Lot data was cleaned up on finalize, so it's okay to call bury
                host.state_mut().bury(token, previous_owner);

                // Refund the bid. If neither bid nor token can not be refunded, error must be returned.
                host.invoke_transfer(&winning_bid.account, winning_bid.amount)?;
            } else {
                let mut owner_share = winning_bid.amount;

                // Transfer royalties. Platform fee is included in the royalty list.
                for share in royalties.iter() {
                    // It is better to ignore errors here:
                    // * Auction function ensures that royalties are under 100%. Bids amount is transferred before
                    //   finalization, so this function should not fail with [TransferError::AmountTooLarge];
                    // * If receiver account doesn't exist, owner_share doesn't get reduced. It is the responsibility of
                    //   NFT contract to provide valid royalties. Since finalize is not allowed to fail, missing accounts
                    //   are just ignored;
                    // * If either problem occurs for any reason, transferring bid and payment should take priority.
                    let _ = transfer_royalty(host, winning_bid.amount, share, &mut owner_share);
                }

                // Log the auction finalization event
                logger.log(&AuctionEvents::finalize(
                    &token.contract,
                    &token.id,
                    &previous_owner,
                    &winning_bid.account,
                    winning_bid.amount,
                    owner_share,
                    &royalties,
                ))?;

                // Transfer price after deducting royalties to owner
                host.invoke_transfer(&previous_owner, owner_share)?;
            }
        }
        AuctionResult::Refund(owner) => {
            // Try to return token to owner if there are no bids
            if nft::transfer(host, &token, &Address::Contract(ctx.self_address()), &owner).is_err()
            {
                // Log the auction abort event
                logger.log(&AuctionEvents::abort(
                    &token.contract,
                    &token.id,
                    &owner,
                    &owner,
                    Amount::zero(),
                ))?;

                // Lot data was cleaned up on finalize, so it's okay to call bury
                host.state_mut().bury(token, owner);
            } else {
                // Log the auction cancel event
                logger.log(&AuctionEvents::cancel(&token.contract, &token.id, &owner))?;
            }
        }
    }

    Ok(())
}

#[receive(
    mutable,
    contract = "BictoryNftAuction",
    name = "cancel",
    parameter = "Token",
    enable_logger
)]
fn contract_cancel<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>>,
    logger: &mut impl HasLogger,
) -> ReceiveResult<()> {
    let token = Token::deserial(&mut ctx.parameter_cursor())?;

    // Only seller or his operator is allowed to call this function
    let sender = if let Address::Account(sender) = ctx.sender() {
        sender
    } else {
        bail!(CustomContractError::OnlyAccountAddress.into());
    };

    // Remove lot with token
    let (owner, bid) = host
        .state_mut()
        .cancel(&token, &sender, ctx.metadata().slot_time())?;

    // Refund last bid
    if let Some(bid) = bid {
        host.invoke_transfer(&bid.account, bid.amount)?;
    }

    // Try to return token to owner
    if nft::transfer(host, &token, &Address::Contract(ctx.self_address()), &owner).is_err() {
        // Log the auction abort event
        logger.log(&AuctionEvents::abort(
            &token.contract,
            &token.id,
            &owner,
            &owner,
            Amount::zero(),
        ))?;

        // Lot data was cleaned up on finalize, so it's okay to call bury
        host.state_mut().bury(token, owner);
    } else {
        // Log the auction cancel event
        logger.log(&AuctionEvents::cancel(&token.contract, &token.id, &owner))?;
    }

    Ok(())
}

#[receive(
    mutable,
    contract = "BictoryNftAuction",
    name = "recover",
    parameter = "Token"
)]
fn contract_recover<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>>,
) -> ReceiveResult<()> {
    let token = Token::deserial(&mut ctx.parameter_cursor())?;

    // Find the token grave
    let owner = host.state_mut().recover(&token)?;

    // Attempt to return token to owner again
    nft::transfer(host, &token, &Address::Contract(ctx.self_address()), &owner)?;

    Ok(())
}

/// Function to manage addresses that are allowed to maintain and modify the state of the contract.
///
///  It rejects if:
///  - Fails to parse `AuthorityUpdateParams` parameters.
///  - If sender is neither one of the admins nor one of the maintainers.
#[receive(
    mutable,
    contract = "BictoryNftAuction",
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
    contract = "BictoryNftAuction",
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

/// Function to update values required for internal contract functionality. This includes:
/// - Royalty. fee percentage for token sale. Gets assigned to a token on mint.
/// - Beneficiary. Account address that receives the fee.
///
///  It rejects if:
///  - Fails to parse `UpdateInternalValueParams` parameters.
///  - If sender is neither one of the admins nor one of the maintainers.
#[receive(
    mutable,
    contract = "BictoryNftAuction",
    name = "updateInternalValue",
    parameter = "InternalValue"
)]
fn update_internal_value<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ReceiveResult<()> {
    if !host.state().authority.has_maintainer_rights(&ctx.sender()) {
        return Err(CustomContractError::Unauthorized.into());
    }

    let mut state = host.state_mut();
    let params = InternalValue::deserial(&mut ctx.parameter_cursor())?;

    match params {
        InternalValue::Royalty(percentage) => state.royalty = percentage,
        InternalValue::Beneficiary(account) => state.beneficiary = account,
    }

    Ok(())
}

/// Function to view values required for internal contract functionality. This includes:
/// - Royalty. fee percentage for token sale. Gets assigned to a token on mint.
/// - Beneficiary. Account address that receives the fee.
///
///  It rejects if:
///  - Fails to parse `ViewInternalValueParams` parameters.
///  - If sender is neither one of the admins nor one of the maintainers.
#[receive(
    contract = "BictoryNftAuction",
    name = "viewInternalValue",
    parameter = "ViewInternalValueParams",
    return_value = "InternalValue"
)]
fn view_internal_value<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &impl HasHost<State<S>, StateApiType = S>,
) -> ReceiveResult<InternalValue> {
    let state = host.state();
    let params = ViewInternalValueParams::deserial(&mut ctx.parameter_cursor())?;

    let value = match params {
        ViewInternalValueParams::Royalty => InternalValue::Royalty(state.royalty),
        ViewInternalValueParams::Beneficiary => InternalValue::Beneficiary(state.beneficiary),
    };

    Ok(value)
}

// Transfer royalty and subtract it from total amount. If transfer fails, `amount` is not changed.
fn transfer_royalty<S: HasStateApi>(
    host: &mut impl HasHost<State<S>>,
    total_price: Amount,
    royalty: &Royalty,
    amount: &mut Amount,
) -> TransferResult {
    let royalty_payout = royalty.percentage * total_price;
    *amount = Amount::from_micro_ccd(
        amount
            .micro_ccd
            .checked_sub(royalty_payout.micro_ccd)
            .ok_or(TransferError::AmountTooLarge)?,
    );
    if royalty_payout > Amount::zero() {
        host.invoke_transfer(&royalty.beneficiary, royalty_payout)?;
    }
    Ok(())
}

#[concordium_cfg_test]
mod tests {
    use super::*;
    //use test_infrastructure::*;

    #[concordium_test]
    fn test_init() {
        todo!()
    }
}
