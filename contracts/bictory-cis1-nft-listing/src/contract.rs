use commons_v1::{
    AuthorityUpdateParams, AuthorityViewParams, CustomContractError, Percentage, Royalty, Token,
};
use concordium_cis1::{OnReceivingCis1Params, TokenIdVec};
use concordium_std::*;

use crate::events::*;
use crate::external::*;
use crate::nft;
use crate::state::{ListingData, State};

/// Initialize the listing contract with an empty list of listings.
#[init(contract = "BictoryNftListing", parameter = "InitParams")]
fn init<S: HasStateApi>(
    ctx: &impl HasInitContext,
    state_builder: &mut StateBuilder<S>,
) -> InitResult<State<S>> {
    let params = InitParams::deserial(&mut ctx.parameter_cursor())?;
    Ok(State::new(
        state_builder,
        params.beneficiary,
        params.percentage,
        ctx.init_origin(),
    ))
}

/// List NFT. This function is intended to be passed as a callback to CIS-1 transfer function.
///
/// While token is listed, listing contract owns the token. Token is transferred to respective account after buy or
/// cancel functions.
#[receive(
    mutable,
    contract = "BictoryNftListing",
    name = "list",
    parameter = "OnReceivingCis1Params<TokenIdVec>",
    enable_logger
)]
fn list<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>>,
    logger: &mut impl HasLogger,
) -> ReceiveResult<()> {
    let transfer_info = OnReceivingCis1Params::<TokenIdVec>::deserial(&mut ctx.parameter_cursor())?;
    // Do not list anything if no tokens were transfered
    if transfer_info.amount == 0 {
        return Ok(());
    }
    // Amount of tokens over 1 is not currently supported
    if transfer_info.amount != 1 {
        return Err(CustomContractError::Unsupported.into());
    }

    let owner = if let Address::Account(owner) = transfer_info.from {
        owner
    } else {
        return Err(CustomContractError::Unsupported.into());
    };

    let contract = if let Address::Contract(sender) = ctx.sender() {
        sender
    } else {
        return Err(CustomContractError::ContractOnly.into());
    };

    let listing_info: ListingInfo = from_bytes(transfer_info.data.as_ref())?;

    let royalties = nft::get_royalties(host, &contract, &transfer_info.token_id)?;
    let platform_fee = host.state().royalty;

    if royalties
        .iter()
        .fold(platform_fee, |acc, x| acc + x.percentage)
        > Percentage::from_percent(100)
    {
        bail!(CustomContractError::InvalidRoyalty.into());
    }

    // Log NFT list event.
    logger.log(&ListingEvent::list(
        &contract,
        &transfer_info.token_id,
        &owner,
        listing_info.price,
        platform_fee,
    ))?;

    host.state_mut().list(
        contract,
        transfer_info.token_id,
        ListingData {
            owner,
            price: listing_info.price,
            platform_fee,
            royalties,
        },
    );

    Ok(())
}

/// Unlist the NFT. After this function token returns to the original owner.
#[receive(
    mutable,
    contract = "BictoryNftListing",
    name = "unlist",
    parameter = "Token",
    enable_logger
)]
fn unlist<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>>,
    logger: &mut impl HasLogger,
) -> ReceiveResult<()> {
    let token: Token = ctx.parameter_cursor().get()?;

    let listing_data = host.state_mut().unlist(&token)?;

    ensure_eq!(
        ctx.sender(),
        Address::Account(listing_data.owner),
        CustomContractError::Unauthorized.into()
    );

    // Log NFT unlist event
    logger.log(&ListingEvent::unlist(
        &token.contract,
        &token.id,
        &listing_data.owner,
    ))?;

    // Return token to owner
    nft::transfer(
        host,
        token,
        Address::Contract(ctx.self_address()),
        listing_data.owner,
    )?;

    Ok(())
}

/// Buy the listed NFT. After this function NFT ownership is transferred to the buyer, token price with royalties
/// deducted is transferred to the lister.
#[receive(
    mutable,
    payable,
    contract = "BictoryNftListing",
    name = "buy",
    parameter = "Token",
    enable_logger
)]
fn buy<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>>,
    amount: Amount,
    logger: &mut impl HasLogger,
) -> ReceiveResult<()> {
    let sender = match ctx.sender() {
        Address::Account(addr) => addr,
        Address::Contract(_) => bail!(CustomContractError::OnlyAccountAddress.into()),
    };
    let token = Token::deserial(&mut ctx.parameter_cursor())?;
    let listing_data = host.state_mut().unlist(&token)?;

    let mut royalties = listing_data.royalties;
    royalties.push(Royalty {
        beneficiary: host.state().beneficiary,
        percentage: listing_data.platform_fee,
    });

    // Owner share to deduct royalties from. The remainder is transferred to the lister.
    let mut owner_share = listing_data.price;

    // Transfer token royalties, reducing owner share
    for share in royalties.iter() {
        transfer_royalty(host, listing_data.price, share, &mut owner_share)?;
    }

    // Log NFT buy event
    logger.log(&ListingEvent::buy(
        &token.contract,
        &token.id,
        &listing_data.owner,
        &sender,
        listing_data.price,
        owner_share,
        &royalties,
    ))?;

    // Transfer price after deducting royalties to owner
    host.invoke_transfer(&listing_data.owner, owner_share)?;

    // Return remaining funds to the buyer
    let remaining_funds = amount - listing_data.price;
    if remaining_funds > Amount::zero() {
        host.invoke_transfer(&sender, remaining_funds)?;
    }

    // Transfer token to buyer
    nft::transfer(host, token, Address::Contract(ctx.self_address()), sender)?;

    Ok(())
}

/// Function to manage addresses that are allowed to maintain and modify the state of the contract.
///
///  It rejects if:
///  - Fails to parse `AuthorityUpdateParams` parameters.
///  - If sender is neither one of the admins nor one of the maintainers.
#[receive(
    mutable,
    contract = "BictoryNftListing",
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
    contract = "BictoryNftListing",
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
    contract = "BictoryNftListing",
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
    contract = "BictoryNftListing",
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
