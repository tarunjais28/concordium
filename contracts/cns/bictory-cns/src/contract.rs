use core::convert::TryFrom;

use commons_v1::{
    AuthorityUpdateParams, AuthorityViewParams, ContractReadError, CustomContractError, DomainKind,
    DomainPrice, HostCnsNftExt, HostCnsOracleExt, HostStorageExt, StorageEntriesRef,
    StorageKeysRef, SubscriptionExpiryStatus,
};
use concordium_cis1::TokenIdVec;
use concordium_std::*;
use sha3::{Digest, Keccak256};

use crate::external::*;
use crate::state::State;
use crate::YEAR_MILLIS;

#[init(contract = "BictoryCns", parameter = "InitParams")]
fn init<S: HasStateApi>(
    ctx: &impl HasInitContext,
    state_builder: &mut StateBuilder<S>,
) -> InitResult<State<S>> {
    let params =
        InitParams::deserial(&mut ctx.parameter_cursor()).map_err(CustomContractError::from)?;

    Ok(State::new(
        state_builder,
        ctx.init_origin(),
        params.registry,
        params.nft,
        params.price_oracle,
        params.subscription_year_limit,
    ))
}

#[receive(
    mutable,
    payable,
    contract = "BictoryCns",
    name = "register",
    parameter = "RegisterParams"
)]
fn register<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
    amount: Amount,
) -> ReceiveResult<()> {
    let params =
        RegisterParams::deserial(&mut ctx.parameter_cursor()).map_err(CustomContractError::from)?;
    let domain = TokenizedDomain::try_from(params.domain.as_ref())?;

    // Only domains can be registered with this function
    ensure!(
        !domain.is_subdomain(),
        CustomContractError::InvalidDomainFormat.into()
    );

    // Ensure registration duration does not exceed `subscription_year_limit` years in the future
    ensure!(
        params.duration_years <= host.state().subscription_year_limit,
        CustomContractError::InvalidDuration.into()
    );

    let namehash = domain.namehash();

    let registry = host.state().registry;
    let nft = host.state().nft;

    let token_id = TokenIdVec(namehash.into());
    let ownership = host
        .cns_nft_get_token_expiry(&nft, token_id.clone())
        .map_err(handle_get_error)?;

    // Check if token exists
    if let Some(ownership_data) = ownership {
        // Check if token has already expired
        ensure!(
            ownership_data.is_expired(),
            CustomContractError::AlreadyExists.into()
        );

        host.cns_nft_burn(&nft, &token_id)
            .map_err(handle_call_error)?;
    }

    let pricing = host
        .cns_get_yearly_domain_price(
            &host.state().price_oracle,
            DomainKind::Domain,
            domain.char_count(),
        )
        .map_err(handle_get_error)?;

    let yearly_price = match pricing {
        DomainPrice::Limited => {
            // Only maintainers and admins are allowed to create domains with limited pricing and registration policy
            ensure!(
                host.state().authority.has_maintainer_rights(&ctx.sender()),
                CustomContractError::Unauthorized.into()
            );
            Amount::zero()
        }
        DomainPrice::Amount(yearly_price) => yearly_price,
    };
    let total_price = yearly_price * params.duration_years as u64;

    // Transfer the fee to the beneficiary
    if total_price != Amount::zero() {
        host.invoke_transfer(&host.state().beneficiary, total_price)?;
    }

    // Refund the remaining CCD if necessary
    if amount - total_price != Amount::zero() {
        host.invoke_transfer(&ctx.invoker(), amount - total_price)?;
    }

    host.cns_nft_mint(
        &nft,
        token_id,
        params.domain,
        ctx.sender(),
        Duration::from_millis(YEAR_MILLIS * params.duration_years as u64),
    )
    .map_err(handle_call_error)?;

    // Insert operation fails with AlreadyExists if entry is present. Token data is burnt without removing the
    // registry data. Try clearing old registry data before inserting new. If NFT was successfully minted, this means
    // that it is okay to remove previous registry data, if present.
    //
    // We ignore errors on remove call because, `CustomContractError::NotFound` is expected to be returned often and
    // should be ignored. `insert` call will produce any other error that we may ignore from `remove` call.
    let _ = host.storage_remove_raw(&registry, &StorageKeysRef::all(namehash.as_slice().into()));
    host.storage_insert(&registry, namehash.as_slice().into(), &(), &params.address)
        .map_err(handle_call_error)?;

    Ok(())
}

#[receive(
    mutable,
    payable,
    contract = "BictoryCns",
    name = "extend",
    parameter = "ExtendParams"
)]
fn extend<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
    amount: Amount,
) -> ReceiveResult<()> {
    let params =
        ExtendParams::deserial(&mut ctx.parameter_cursor()).map_err(CustomContractError::from)?;
    let domain = TokenizedDomain::try_from(params.domain.as_ref())?;

    // Only domains can be registered with this function
    ensure!(
        !domain.is_subdomain(),
        CustomContractError::InvalidDomainFormat.into()
    );

    let namehash = domain.namehash();
    let token_id = TokenIdVec(namehash.into());
    let nft = host.state().nft;

    // Check if token exists
    let ownership = host
        .cns_nft_get_token_expiry(&nft, token_id.clone())
        .map_err(handle_get_error)?
        .ok_or(CustomContractError::NotFound)?;

    let extension_duration = Duration::from_millis(YEAR_MILLIS * params.duration_years as u64);
    let subscription_duration_limit =
        Duration::from_millis(YEAR_MILLIS * host.state().subscription_year_limit as u64);

    // Ensure extention does not exceed `subscription_year_limit` years in the future and the token is not expired
    match ownership.expiry {
        SubscriptionExpiryStatus::Owned(timestamp) => ensure!(
            timestamp
                .checked_add(extension_duration)
                .ok_or(CustomContractError::InvalidDuration)?
                .duration_between(ctx.metadata().slot_time())
                <= subscription_duration_limit,
            CustomContractError::InvalidDuration.into()
        ),
        SubscriptionExpiryStatus::Grace(_) => {
            ensure!(extension_duration <= subscription_duration_limit);
        }
        SubscriptionExpiryStatus::Expired => return Err(CustomContractError::NotFound.into()),
    };

    let pricing = host
        .cns_get_yearly_domain_price(
            &host.state().price_oracle,
            DomainKind::Domain,
            domain.char_count(),
        )
        .map_err(handle_get_error)?;

    let yearly_price = match pricing {
        DomainPrice::Limited => {
            // Only maintainers and admins are allowed to extend domains with limited pricing and registration policy
            ensure!(
                host.state().authority.has_maintainer_rights(&ctx.sender()),
                CustomContractError::Unauthorized.into()
            );
            Amount::zero()
        }
        DomainPrice::Amount(yearly_price) => yearly_price,
    };
    let total_price = yearly_price * params.duration_years as u64;

    // Transfer the fee to the beneficiary
    if total_price != Amount::zero() {
        host.invoke_transfer(&host.state().beneficiary, total_price)?;
    }

    // Lend the NFT
    host.cns_nft_lend(&nft, token_id, extension_duration)
        .map_err(handle_call_error)?;

    // Refund the remaining CCD if necessary
    if amount - total_price != Amount::zero() {
        host.invoke_transfer(&ctx.invoker(), amount - total_price)?;
    }

    Ok(())
}

#[receive(
    mutable,
    contract = "BictoryCns",
    name = "setAddress",
    parameter = "SetAddressParams"
)]
fn set_address<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ReceiveResult<()> {
    let params = SetAddressParams::deserial(&mut ctx.parameter_cursor())
        .map_err(CustomContractError::from)?;
    let domain = TokenizedDomain::try_from(params.domain.as_ref())?;
    let (domain_namehash, subdomain_namehash) = domain.domain_subdomain_namehashes();
    let token_id = TokenIdVec(domain_namehash.into());
    let registry = host.state().registry;

    let subscription_status = host
        .cns_nft_get_token_expiry(&host.state().nft, token_id)
        .map_err(handle_get_error)?
        .ok_or(CustomContractError::NotFound)?;

    match subscription_status.expiry {
        SubscriptionExpiryStatus::Owned(_) if subscription_status.owner == ctx.sender() => (),
        SubscriptionExpiryStatus::Expired => return Err(CustomContractError::NotFound.into()),
        _ => return Err(CustomContractError::Unauthorized.into()),
    }

    host.storage_update(
        &registry,
        subdomain_namehash.as_slice().into(),
        &(),
        &params.address,
    )
    .map_err(handle_call_error)?;

    // TODO: log AddressChanged event

    Ok(())
}

#[receive(
    contract = "BictoryCns",
    name = "resolve",
    parameter = "ResolveParams",
    return_value = "Address"
)]
fn resolve<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &impl HasHost<State<S>, StateApiType = S>,
) -> ReceiveResult<Address> {
    let params =
        ResolveParams::deserial(&mut ctx.parameter_cursor()).map_err(CustomContractError::from)?;
    let domain = TokenizedDomain::try_from(params.domain.as_ref())?;
    let (domain_namehash, subdomain_namehash) = domain.domain_subdomain_namehashes();
    let state = host.state();

    let ownership_data = host
        .cns_nft_get_token_expiry(&state.nft, TokenIdVec(domain_namehash.into()))
        .map_err(handle_get_error)?
        .ok_or(CustomContractError::NotFound)?;

    ensure!(
        ownership_data.is_owned(),
        CustomContractError::NotFound.into()
    );

    let address: Option<Address> = host
        .storage_get(&state.registry, subdomain_namehash.as_slice().into(), &())
        .map_err(handle_get_error)?;

    address.ok_or(CustomContractError::NotFound.into())
}

#[receive(
    mutable,
    contract = "BictoryCns",
    name = "setData",
    parameter = "SetDataParams"
)]
fn set_data<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ReceiveResult<()> {
    let params =
        SetDataParams::deserial(&mut ctx.parameter_cursor()).map_err(CustomContractError::from)?;
    ensure!(
        !params.key.is_empty(),
        CustomContractError::ParseParams.into()
    );

    let domain = TokenizedDomain::try_from(params.domain.as_ref())?;
    let (domain_namehash, subdomain_namehash) = domain.domain_subdomain_namehashes();
    let token_id = TokenIdVec(domain_namehash.into());
    let registry = host.state().registry;

    let subscription_status = host
        .cns_nft_get_token_expiry(&host.state().nft, token_id)
        .map_err(handle_get_error)?
        .ok_or(CustomContractError::NotFound)?;

    match subscription_status.expiry {
        SubscriptionExpiryStatus::Owned(_) if subscription_status.owner == ctx.sender() => (),
        SubscriptionExpiryStatus::Expired => return Err(CustomContractError::NotFound.into()),
        _ => return Err(CustomContractError::Unauthorized.into()),
    }

    match params.value {
        DataValue::Empty => host
            .storage_remove(&registry, subdomain_namehash.as_slice().into(), &params.key)
            .map_err(handle_call_error)?,
        v => host
            .storage_update(
                &registry,
                subdomain_namehash.as_slice().into(),
                &params.key,
                &v,
            )
            .map_err(handle_call_error)?,
    }

    // TODO: log DataChanged event

    Ok(())
}

#[receive(
    contract = "BictoryCns",
    name = "getData",
    parameter = "GetDataParams",
    return_value = "DataValue"
)]
fn get_data<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &impl HasHost<State<S>, StateApiType = S>,
) -> ReceiveResult<DataValue> {
    let params =
        GetDataParams::deserial(&mut ctx.parameter_cursor()).map_err(CustomContractError::from)?;
    ensure!(
        !params.key.is_empty(),
        CustomContractError::ParseParams.into()
    );

    let domain = TokenizedDomain::try_from(params.domain.as_ref())?;
    let (domain_namehash, subdomain_namehash) = domain.domain_subdomain_namehashes();
    let state = host.state();

    let ownership_data = host
        .cns_nft_get_token_expiry(&state.nft, TokenIdVec(domain_namehash.into()))
        .map_err(handle_get_error)?
        .ok_or(CustomContractError::NotFound)?;

    ensure!(
        ownership_data.is_owned(),
        CustomContractError::NotFound.into()
    );

    let data: Option<DataValue> = host
        .storage_get(
            &state.registry,
            subdomain_namehash.as_slice().into(),
            &params.key,
        )
        .map_err(handle_get_error)?;

    data.ok_or(CustomContractError::NotFound.into())
}

#[receive(
    mutable,
    payable,
    contract = "BictoryCns",
    name = "createSubdomain",
    parameter = "SubdomainParams"
)]
fn create_subdomain<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
    amount: Amount,
) -> ReceiveResult<()> {
    let params = SubdomainParams::deserial(&mut ctx.parameter_cursor())
        .map_err(CustomContractError::from)?;
    let registry = host.state().registry;
    let domain = TokenizedDomain::try_from(params.subdomain.as_ref())?;

    let (domain_namehash, subdomain_namehash) = domain.domain_subdomain_namehashes();

    let subscription_status = host
        .cns_nft_get_token_expiry(&host.state().nft, TokenIdVec(domain_namehash.into()))
        .map_err(handle_get_error)?
        .ok_or(CustomContractError::NotFound)?;

    match subscription_status.expiry {
        SubscriptionExpiryStatus::Owned(_) if subscription_status.owner == ctx.sender() => (),
        SubscriptionExpiryStatus::Expired => return Err(CustomContractError::NotFound.into()),
        _ => return Err(CustomContractError::Unauthorized.into()),
    }

    let pricing = host
        .cns_get_yearly_domain_price(
            &host.state().price_oracle,
            DomainKind::Subdomain,
            params.subdomain.len() as u16,
        )
        .map_err(handle_get_error)?;

    let total_price = match pricing {
        DomainPrice::Limited => {
            // Only maintainers and admins are allowed to create domains with limited pricing and registration policy
            ensure!(
                host.state().authority.has_maintainer_rights(&ctx.sender()),
                CustomContractError::Unauthorized.into()
            );
            Amount::zero()
        }
        DomainPrice::Amount(yearly_price) => yearly_price,
    };

    // Transfer the fee to the beneficiary
    if total_price != Amount::zero() {
        host.invoke_transfer(&host.state().beneficiary, total_price)?;
    }

    // Refund the remaining CCD if necessary
    if amount - total_price != Amount::zero() {
        host.invoke_transfer(&ctx.invoker(), amount - total_price)?;
    }

    // Insert operation fails with AlreadyExists if entry is present.
    host.storage_insert_raw(
        &registry,
        &StorageEntriesRef {
            prefix: subdomain_namehash.as_slice().into(),
            entries: &[],
        },
    )
    .map_err(handle_call_error)?;

    Ok(())
}

#[receive(
    mutable,
    contract = "BictoryCns",
    name = "deleteSubdomain",
    parameter = "SubdomainParams"
)]
fn delete_subdomain<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ReceiveResult<()> {
    let params = SubdomainParams::deserial(&mut ctx.parameter_cursor())
        .map_err(CustomContractError::from)?;
    let registry = host.state().registry;
    let domain = TokenizedDomain::try_from(params.subdomain.as_ref())?;
    ensure!(
        domain.is_subdomain(),
        CustomContractError::InvalidDomainFormat.into()
    );
    let (domain_namehash, subdomain_namehash) = domain.domain_subdomain_namehashes();

    if let Some(ownership_data) = host
        .cns_nft_get_token_expiry(&host.state().nft, TokenIdVec(domain_namehash.into()))
        .map_err(handle_get_error)?
    {
        ensure!(
            ownership_data.is_owned_by(ctx.sender()) || ownership_data.is_expired(),
            CustomContractError::Unauthorized.into()
        );
    }

    // Insert operation fails with AlreadyExists if entry is present.
    host.storage_remove_raw(
        &registry,
        &StorageKeysRef::all(subdomain_namehash.as_slice().into()),
    )
    .map_err(handle_call_error)?;

    Ok(())
}

#[receive(
    mutable,
    contract = "BictoryCns",
    name = "updateAuthority",
    parameter = "AuthorityUpdateParams"
)]
fn update_authority<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ReceiveResult<()> {
    let state = host.state_mut();
    let params = AuthorityUpdateParams::deserial(&mut ctx.parameter_cursor())
        .map_err(CustomContractError::from)?;
    let sender = ctx.sender();
    state.authority.handle_update(sender, params)
}

#[receive(
    contract = "BictoryCns",
    name = "viewAuthority",
    parameter = "AuthorityViewParams",
    return_value = "Vec<Address>"
)]
fn view_authority<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &impl HasHost<State<S>, StateApiType = S>,
) -> ReceiveResult<Vec<Address>> {
    let params = AuthorityViewParams::deserial(&mut ctx.parameter_cursor())
        .map_err(CustomContractError::from)?;
    Ok(host.state().authority.handle_view(params))
}

#[receive(
    mutable,
    contract = "BictoryCns",
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

    let params =
        InternalValue::deserial(&mut ctx.parameter_cursor()).map_err(CustomContractError::from)?;

    let mut state = host.state_mut();
    match params {
        InternalValue::CnsNft(nft) => state.nft = nft,
        InternalValue::Oracle(oracle) => state.price_oracle = oracle,
        InternalValue::Beneficiary(beneficiary) => state.beneficiary = beneficiary,
        InternalValue::SubscriptionYearLimit(limit) => state.subscription_year_limit = limit,
    }

    Ok(())
}

#[receive(
    contract = "BictoryCns",
    name = "viewInternalValue",
    parameter = "InternalViewParams",
    return_value = "InternalValue"
)]
fn view_internal_value<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &impl HasHost<State<S>, StateApiType = S>,
) -> ReceiveResult<InternalValue> {
    let params = InternalViewParams::deserial(&mut ctx.parameter_cursor())
        .map_err(CustomContractError::from)?;

    let state = host.state();
    let address = match params {
        InternalViewParams::CnsNft => InternalValue::CnsNft(state.nft),
        InternalViewParams::Oracle => InternalValue::Oracle(state.price_oracle),
        InternalViewParams::Beneficiary => InternalValue::Beneficiary(state.beneficiary),
        InternalViewParams::SubscriptionYearLimit => {
            InternalValue::SubscriptionYearLimit(state.subscription_year_limit)
        }
    };

    Ok(address)
}

struct TokenizedDomain<'a> {
    domain: &'a str,
    labels: Vec<&'a str>,
}

impl<'a> TryFrom<&'a str> for TokenizedDomain<'a> {
    type Error = CustomContractError;

    fn try_from(domain: &'a str) -> Result<Self, Self::Error> {
        // Domain must be under 256 bytes
        ensure!(domain.len() < 256, CustomContractError::InvalidDomainFormat);

        let mut labels = domain.split('.').rev();

        // Root domain must be ccd. Check it and skip
        ensure_eq!(
            labels.next(),
            Some("ccd"),
            CustomContractError::InvalidDomainFormat
        );

        let domain = labels
            .next()
            .ok_or(CustomContractError::InvalidDomainFormat)?;

        let labels: Vec<&str> = labels.collect();

        // Must be at least one label, each label must not be empty, but under 64 bytes
        ensure!(
            labels
                .iter()
                .all(|label| !label.is_empty() && label.len() < 64),
            CustomContractError::InvalidDomainFormat
        );

        Ok(TokenizedDomain { domain, labels })
    }
}

const CCD_HASH: [u8; 32] = [
    0x53, 0xbb, 0xd6, 0xc8, 0xc1, 0xbd, 0xc5, 0xc6, 0x28, 0x27, 0x1c, 0x55, 0xa6, 0xac, 0x73, 0xa1,
    0xe9, 0x7a, 0xfb, 0xb1, 0x4d, 0x4a, 0xeb, 0x3a, 0xdd, 0xb8, 0xb7, 0xb8, 0x0e, 0x4f, 0x45, 0x5a,
];

impl<'a> TokenizedDomain<'a> {
    fn domain_subdomain_namehashes(&self) -> ([u8; 32], [u8; 32]) {
        let domain_namehash = namehash_label(CCD_HASH, self.domain);
        let subdomain_namehash = self
            .labels
            .iter()
            .copied()
            .fold(domain_namehash, namehash_label);
        (domain_namehash, subdomain_namehash)
    }

    fn namehash(&self) -> [u8; 32] {
        self.labels
            .iter()
            .copied()
            .fold(namehash_label(CCD_HASH, self.domain), namehash_label)
    }

    fn char_count(&self) -> u16 {
        if self.is_subdomain() {
            (self.labels.iter().fold(0, |acc, x| acc + x.chars().count())
                + self.labels.len().saturating_sub(1)) as u16
        } else {
            self.domain.chars().count() as u16
        }
    }

    fn is_subdomain(&self) -> bool {
        !self.labels.is_empty()
    }
}

fn namehash_label(namehash: [u8; 32], label: &str) -> [u8; 32] {
    let mut hasher = Keccak256::default();
    hasher.update(label.as_bytes());
    let labelhash = hasher.finalize_reset();
    hasher.update(namehash);
    hasher.update(labelhash);
    hasher.finalize_reset().into()
}

fn handle_call_error<R>(error: CallContractError<R>) -> Reject {
    match error {
        CallContractError::LogicReject { reason, .. } => match reason {
            // CustomContractError::ParseParams | concordium ParseError
            -1 | -2147483646 => CustomContractError::Incompatible.into(),
            // CustomContractError::NotFound
            -30 => CustomContractError::NotFound.into(),
            // CustomContractError::AlreadyExists
            -35 => CustomContractError::AlreadyExists.into(),
            // CustomContractError::Unauthorized, happens if CNS contract was not authorized
            -36 => CustomContractError::OperationNotPermitted.into(),
            // Remaining errors
            _ => CustomContractError::InvokeContractError.into(),
        },
        e => e.into(),
    }
}

fn handle_get_error<R>(error: ContractReadError<R>) -> Reject {
    match error {
        ContractReadError::Call(e) => handle_call_error(e),
        ContractReadError::Compatibility => CustomContractError::Incompatible.into(),
        ContractReadError::Parse => CustomContractError::InvokeContractError.into(),
    }
}

#[concordium_cfg_test]
mod tests {
    use super::*;
    use commons_v1::test::*;
    use commons_v1::{
        Bytes, CnsMintParams, GetDomainPriceParams, GetDomainPriceResult, LendParams,
        MaybeStorageEntry, StorageEntries, StorageGetEntryResult, StorageKeySelection, StorageKeys,
        TokenParams, TokenSubscriptionStatus,
    };
    use concordium_std::test_infrastructure::*;

    const TEST_YEARLY_DOMAIN_PRICE: Amount = Amount::from_ccd(10);
    const TEST_YEARLY_SUBDOMAIN_PRICE: Amount = Amount::from_ccd(5);

    const REGISTRY: ContractAddress = ContractAddress {
        index: 1,
        subindex: 0,
    };
    const CNS_NFT: ContractAddress = ContractAddress {
        index: 2,
        subindex: 0,
    };
    const PRICE_ORACLE: ContractAddress = ContractAddress {
        index: 3,
        subindex: 0,
    };

    const ADMIN: AccountAddress = AccountAddress([1; 32]);
    const MAINTAINER: AccountAddress = AccountAddress([2; 32]);

    const USER_1: AccountAddress = AccountAddress([16; 32]);
    const USER_2: AccountAddress = AccountAddress([17; 32]);

    fn test_slot_time() -> Timestamp {
        Timestamp::from_timestamp_millis(YEAR_MILLIS * 10)
    }

    fn default_host() -> TestHost<State<TestStateApi>> {
        let mut ctx = TestInitContext::empty();
        // admin is initialized to `ctx.origin()`
        let params = InitParams {
            registry: REGISTRY,
            nft: CNS_NFT,
            price_oracle: PRICE_ORACLE,
            subscription_year_limit: 3,
        };
        let bytes = to_bytes(&params);
        ctx.set_init_origin(ADMIN).set_parameter(&bytes);
        let mut state_builder = TestStateBuilder::new();

        // Call the init method.
        let state = init(&ctx, &mut state_builder).expect_report("Failed during init_BictoryCns");

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

        host
    }

    fn test_price_oracle(params: &GetDomainPriceParams) -> Option<GetDomainPriceResult> {
        let result = match params.domain_kind {
            DomainKind::Domain if params.length <= 3 => DomainPrice::Limited,
            DomainKind::Domain => DomainPrice::Amount(TEST_YEARLY_DOMAIN_PRICE),
            DomainKind::Subdomain => DomainPrice::Amount(TEST_YEARLY_SUBDOMAIN_PRICE),
        };
        Some(GetDomainPriceResult { result })
    }

    #[concordium_test]
    fn test_namehash() {
        let cases = &[
            (
                "bar.ccd",
                [
                    0xfb, 0xf1, 0xc5, 0x6a, 0x2c, 0xad, 0x10, 0xf3, 0x9d, 0xb7, 0x80, 0x93, 0x40,
                    0xe8, 0x86, 0x4b, 0xa3, 0x18, 0xc3, 0x98, 0xfd, 0x30, 0x96, 0x9a, 0x8b, 0x7b,
                    0x0d, 0xb7, 0x1a, 0x44, 0xa9, 0xf9,
                ],
            ),
            (
                "foo.bar.ccd",
                [
                    0x54, 0x3f, 0xd6, 0x52, 0x1c, 0x16, 0x96, 0xb1, 0x2b, 0x10, 0x83, 0x16, 0xbe,
                    0x60, 0x7b, 0x75, 0xbd, 0x01, 0xbb, 0xe1, 0xf2, 0x48, 0xcc, 0x85, 0x14, 0x24,
                    0x18, 0x20, 0xe3, 0xf5, 0xd0, 0x2b,
                ],
            ),
            (
                "test.ccd",
                [
                    173, 123, 180, 135, 98, 0, 156, 153, 206, 68, 166, 215, 247, 255, 219, 75, 147,
                    41, 70, 156, 250, 132, 142, 41, 245, 206, 153, 251, 78, 159, 128, 228,
                ],
            ),
            (
                "test.test.ccd",
                [
                    215, 157, 31, 243, 113, 123, 47, 160, 239, 78, 213, 41, 86, 187, 11, 181, 29,
                    236, 105, 171, 172, 159, 39, 126, 151, 86, 194, 67, 114, 59, 241, 157,
                ],
            ),
        ];

        for (name, expected_namehash) in cases {
            let domain = TokenizedDomain::try_from(*name).expect_report("Unable to parse domain");
            let namehash = domain.namehash();
            claim_eq!(namehash, *expected_namehash);
        }
    }

    #[concordium_test]
    fn test_char_count() {
        let cases = &[
            ("foo.ccd", 3u16),
            ("тест.ccd", 4),
            ("💎.ccd", 1),
            ("汉语.ccd", 2),
            // Only subdomain characters have to be counted, if it's a subdomain
            ("bar.foo.ccd", 3u16),
            // Total subdomain length has to be counted, because parent subdomains do not have own NFT token
            ("baz.bar.foo.ccd", 7u16),
        ];

        for (name, actual_count) in cases {
            let domain = TokenizedDomain::try_from(*name).expect_report("Unable to parse domain");
            let char_count = domain.char_count();
            claim_eq!(char_count, *actual_count);
        }
    }

    #[concordium_test]
    fn test_init_test_state() {
        let host = default_host();
        let state = host.state();

        // Assert properties
        claim_eq!(state.registry, REGISTRY);
        claim_eq!(state.nft, CNS_NFT);
        claim_eq!(state.price_oracle, PRICE_ORACLE);

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
    fn test_register_new() {
        let mut host = default_host();

        let mut ctx = TestReceiveContext::empty();
        let params = RegisterParams {
            domain: String::from("test.ccd"),
            address: Address::Account(USER_1),
            duration_years: 2,
        };
        let bytes = to_bytes(&params);
        ctx.set_sender(Address::Account(USER_1))
            .set_invoker(USER_1)
            .set_parameter(&bytes)
            .set_metadata_slot_time(test_slot_time());
        // Get pricing info
        host.setup_mock_entrypoint(
            PRICE_ORACLE,
            OwnedEntrypointName::new_unchecked(String::from("getYearlyDomainPrice")),
            parse_and_map_mock(test_price_oracle),
        );
        // Get ownership info
        host.setup_mock_entrypoint(
            CNS_NFT,
            OwnedEntrypointName::new_unchecked(String::from("getTokenExpiry")),
            parse_and_ok_mock::<TokenParams, _>(None::<TokenSubscriptionStatus>),
        );
        // Mint token on success
        host.setup_mock_entrypoint(
            CNS_NFT,
            OwnedEntrypointName::new_unchecked(String::from("mint")),
            parse_and_ok_mock::<CnsMintParams, _>(()),
        );
        // Try removing old registry data
        host.setup_mock_entrypoint(
            REGISTRY,
            OwnedEntrypointName::new_unchecked(String::from("remove")),
            parse_and_ok_mock::<StorageEntries, _>(()),
        );
        // Update registry data
        host.setup_mock_entrypoint(
            REGISTRY,
            OwnedEntrypointName::new_unchecked(String::from("insert")),
            parse_and_ok_mock::<StorageEntries, _>(()),
        );
        let invoke_amount = TEST_YEARLY_DOMAIN_PRICE * 2 + Amount::from_ccd(2);
        host.set_self_balance(invoke_amount);
        let result = register(&ctx, &mut host, invoke_amount);

        claim_eq!(result, Ok(()));
        // Transfer subscription cost
        claim!(host.transfer_occurred(&ADMIN, TEST_YEARLY_DOMAIN_PRICE * 2));
        // Return extra
        claim!(host.transfer_occurred(&USER_1, Amount::from_ccd(2)));
    }

    #[concordium_test]
    fn test_register_expired() {
        let mut host = default_host();

        let mut ctx = TestReceiveContext::empty();
        let params = RegisterParams {
            domain: String::from("test.ccd"),
            address: Address::Account(USER_1),
            duration_years: 2,
        };
        let bytes = to_bytes(&params);
        ctx.set_sender(Address::Account(USER_1))
            .set_invoker(USER_1)
            .set_parameter(&bytes)
            .set_metadata_slot_time(test_slot_time());
        // Get pricing info
        host.setup_mock_entrypoint(
            PRICE_ORACLE,
            OwnedEntrypointName::new_unchecked(String::from("getYearlyDomainPrice")),
            parse_and_map_mock(test_price_oracle),
        );
        // Get ownership info
        host.setup_mock_entrypoint(
            CNS_NFT,
            OwnedEntrypointName::new_unchecked(String::from("getTokenExpiry")),
            parse_and_ok_mock::<TokenParams, _>(Some(TokenSubscriptionStatus {
                owner: Address::Account(USER_2),
                expiry: SubscriptionExpiryStatus::Expired,
            })),
        );
        // Burn expired token
        host.setup_mock_entrypoint(
            CNS_NFT,
            OwnedEntrypointName::new_unchecked(String::from("burn")),
            parse_and_ok_mock::<TokenIdVec, _>(()),
        );
        // Clear registry data
        host.setup_mock_entrypoint(
            REGISTRY,
            OwnedEntrypointName::new_unchecked(String::from("remove")),
            parse_and_ok_mock::<StorageKeys, _>(()),
        );
        // Mint token on success
        host.setup_mock_entrypoint(
            CNS_NFT,
            OwnedEntrypointName::new_unchecked(String::from("mint")),
            parse_and_ok_mock::<CnsMintParams, _>(()),
        );
        // Update registry data
        host.setup_mock_entrypoint(
            REGISTRY,
            OwnedEntrypointName::new_unchecked(String::from("insert")),
            parse_and_ok_mock::<StorageEntries, _>(()),
        );
        let invoke_amount = TEST_YEARLY_DOMAIN_PRICE * 2 + Amount::from_ccd(2);
        host.set_self_balance(invoke_amount);
        let result = register(&ctx, &mut host, invoke_amount);

        claim_eq!(result, Ok(()));
        // Transfer subscription cost
        claim!(host.transfer_occurred(&ADMIN, TEST_YEARLY_DOMAIN_PRICE * 2));
        // Return extra
        claim!(host.transfer_occurred(&USER_1, Amount::from_ccd(2)));
    }

    #[concordium_test]
    fn test_register_grace() {
        let mut host = default_host();

        let mut ctx = TestReceiveContext::empty();
        let params = RegisterParams {
            domain: String::from("test.ccd"),
            address: Address::Account(USER_1),
            duration_years: 2,
        };
        let bytes = to_bytes(&params);
        ctx.set_sender(Address::Account(USER_1))
            .set_invoker(USER_1)
            .set_parameter(&bytes)
            .set_metadata_slot_time(test_slot_time());
        // Get ownership info
        host.setup_mock_entrypoint(
            CNS_NFT,
            OwnedEntrypointName::new_unchecked(String::from("getTokenExpiry")),
            parse_and_ok_mock::<TokenParams, _>(Some(TokenSubscriptionStatus {
                owner: Address::Account(USER_2),
                expiry: SubscriptionExpiryStatus::Grace(
                    test_slot_time()
                        .checked_add(Duration::from_days(50))
                        .unwrap(),
                ),
            })),
        );
        let invoke_amount = TEST_YEARLY_DOMAIN_PRICE * 2 + Amount::from_ccd(2);
        host.set_self_balance(invoke_amount);
        let result = register(&ctx, &mut host, invoke_amount);

        claim_eq!(result, Err(CustomContractError::AlreadyExists.into()));
    }

    #[concordium_test]
    fn test_extend() {
        let mut host = default_host();

        let mut ctx = TestReceiveContext::empty();
        let params = ExtendParams {
            domain: String::from("test.ccd"),
            duration_years: 2,
        };
        let bytes = to_bytes(&params);
        ctx.set_sender(Address::Account(USER_2))
            .set_invoker(USER_2)
            .set_parameter(&bytes)
            .set_metadata_slot_time(test_slot_time());
        host.setup_mock_entrypoint(
            CNS_NFT,
            OwnedEntrypointName::new_unchecked(String::from("getTokenExpiry")),
            parse_and_ok_mock::<TokenParams, _>(Some(TokenSubscriptionStatus {
                owner: Address::Account(USER_1),
                expiry: SubscriptionExpiryStatus::Owned(
                    test_slot_time()
                        .checked_add(Duration::from_millis(YEAR_MILLIS))
                        .unwrap(),
                ),
            })),
        );
        host.setup_mock_entrypoint(
            PRICE_ORACLE,
            OwnedEntrypointName::new_unchecked(String::from("getYearlyDomainPrice")),
            parse_and_map_mock(test_price_oracle),
        );
        host.setup_mock_entrypoint(
            CNS_NFT,
            OwnedEntrypointName::new_unchecked(String::from("lend")),
            parse_and_ok_mock::<LendParams, _>(()),
        );

        let invoke_amount = TEST_YEARLY_DOMAIN_PRICE * 2 + Amount::from_ccd(2);
        host.set_self_balance(invoke_amount);

        let result = extend(&ctx, &mut host, invoke_amount);

        claim_eq!(result, Ok(()));
        // Transfer subscription cost
        claim!(host.transfer_occurred(&ADMIN, TEST_YEARLY_DOMAIN_PRICE * 2));
        // Return extra
        claim!(host.transfer_occurred(&USER_2, Amount::from_ccd(2)));
    }

    #[concordium_test]
    fn test_set_address() {
        let mut host = default_host();

        let mut ctx = TestReceiveContext::empty();
        let params = SetAddressParams {
            domain: String::from("test.ccd"),
            address: Address::Account(USER_1),
        };
        let bytes = to_bytes(&params);
        ctx.set_sender(Address::Account(USER_1))
            .set_parameter(&bytes);
        host.setup_mock_entrypoint(
            CNS_NFT,
            OwnedEntrypointName::new_unchecked(String::from("getTokenExpiry")),
            parse_and_ok_mock::<TokenParams, _>(Some(TokenSubscriptionStatus {
                owner: Address::Account(USER_1),
                expiry: SubscriptionExpiryStatus::Owned(
                    test_slot_time()
                        .checked_add(Duration::from_millis(YEAR_MILLIS))
                        .unwrap(),
                ),
            })),
        );
        host.setup_mock_entrypoint(
            REGISTRY,
            OwnedEntrypointName::new_unchecked(String::from("update")),
            parse_and_ok_mock::<StorageEntries, _>(()),
        );
        let result = set_address(&ctx, &mut host);
        claim_eq!(result, Ok(()));
    }

    #[concordium_test]
    fn test_resolve() {
        let mut host = default_host();

        let mut ctx = TestReceiveContext::empty();
        let params = ResolveParams {
            domain: String::from("test.ccd"),
        };
        let bytes = to_bytes(&params);
        ctx.set_sender(Address::Account(USER_2))
            .set_parameter(&bytes);
        // Get ownership info
        host.setup_mock_entrypoint(
            CNS_NFT,
            OwnedEntrypointName::new_unchecked(String::from("getTokenExpiry")),
            parse_and_ok_mock::<TokenParams, _>(Some(TokenSubscriptionStatus {
                owner: Address::Account(USER_2),
                expiry: SubscriptionExpiryStatus::Owned(
                    test_slot_time()
                        .checked_add(Duration::from_days(50))
                        .unwrap(),
                ),
            })),
        );
        // Get registry data
        host.setup_mock_entrypoint(
            REGISTRY,
            OwnedEntrypointName::new_unchecked(String::from("get")),
            parse_and_ok_mock::<StorageKeys, _>(Some(StorageGetEntryResult {
                prefix: Bytes(
                    TokenizedDomain::try_from(params.domain.as_ref())
                        .expect_report("Unable to parse domain")
                        .namehash()
                        .into(),
                ),
                entries: vec![MaybeStorageEntry {
                    key: Bytes::from([]),
                    value: Some(Bytes(to_bytes(&Address::Account(USER_1)))),
                }],
            })),
        );
        let result = resolve(&ctx, &mut host);
        claim_eq!(result, Ok(Address::Account(USER_1)));
    }

    #[concordium_test]
    fn test_resolve_expired() {
        let mut host = default_host();

        let mut ctx = TestReceiveContext::empty();
        let params = ResolveParams {
            domain: String::from("test.ccd"),
        };
        let bytes = to_bytes(&params);
        ctx.set_sender(Address::Account(USER_2))
            .set_parameter(&bytes);
        // Get ownership info
        host.setup_mock_entrypoint(
            CNS_NFT,
            OwnedEntrypointName::new_unchecked(String::from("getTokenExpiry")),
            parse_and_ok_mock::<TokenParams, _>(Some(TokenSubscriptionStatus {
                owner: Address::Account(USER_2),
                expiry: SubscriptionExpiryStatus::Expired,
            })),
        );
        let result = resolve(&ctx, &mut host);
        claim_eq!(result, Err(CustomContractError::NotFound.into()));
    }

    #[concordium_test]
    fn test_resolve_missing() {
        let mut host = default_host();

        // Resolve missing
        let mut ctx = TestReceiveContext::empty();
        let params = ResolveParams {
            domain: String::from("test.ccd"),
        };
        let bytes = to_bytes(&params);
        ctx.set_sender(Address::Account(USER_2))
            .set_parameter(&bytes);
        host.setup_mock_entrypoint(
            CNS_NFT,
            OwnedEntrypointName::new_unchecked(String::from("getTokenExpiry")),
            parse_and_ok_mock::<TokenParams, _>(None::<TokenSubscriptionStatus>),
        );
        let result = resolve(&ctx, &mut host);
        claim_eq!(result, Err(CustomContractError::NotFound.into()));
    }

    #[concordium_test]
    fn test_set_data() {
        let mut host = default_host();

        let mut ctx = TestReceiveContext::empty();
        let params = SetDataParams {
            domain: String::from("test.ccd"),
            key: String::from("Twitter"),
            value: DataValue::Url(String::from("https://twitter.com/cns-test")),
        };
        let bytes = to_bytes(&params);
        ctx.set_sender(Address::Account(USER_1))
            .set_parameter(&bytes);
        host.setup_mock_entrypoint(
            CNS_NFT,
            OwnedEntrypointName::new_unchecked(String::from("getTokenExpiry")),
            parse_and_ok_mock::<TokenParams, _>(Some(TokenSubscriptionStatus {
                owner: Address::Account(USER_1),
                expiry: SubscriptionExpiryStatus::Owned(
                    test_slot_time()
                        .checked_add(Duration::from_millis(YEAR_MILLIS))
                        .unwrap(),
                ),
            })),
        );
        host.setup_mock_entrypoint(
            REGISTRY,
            OwnedEntrypointName::new_unchecked(String::from("update")),
            parse_and_ok_mock::<StorageEntries, _>(()),
        );
        let result = set_data(&ctx, &mut host);
        claim_eq!(result, Ok(()));
    }

    #[concordium_test]
    fn test_unset_data() {
        let mut host = default_host();

        let mut ctx = TestReceiveContext::empty();
        let params = SetDataParams {
            domain: String::from("test.ccd"),
            key: String::from("Twitter"),
            value: DataValue::Empty,
        };
        let bytes = to_bytes(&params);
        ctx.set_sender(Address::Account(USER_1))
            .set_parameter(&bytes);
        host.setup_mock_entrypoint(
            CNS_NFT,
            OwnedEntrypointName::new_unchecked(String::from("getTokenExpiry")),
            parse_and_ok_mock::<TokenParams, _>(Some(TokenSubscriptionStatus {
                owner: Address::Account(USER_1),
                expiry: SubscriptionExpiryStatus::Owned(
                    test_slot_time()
                        .checked_add(Duration::from_millis(YEAR_MILLIS))
                        .unwrap(),
                ),
            })),
        );
        host.setup_mock_entrypoint(
            REGISTRY,
            OwnedEntrypointName::new_unchecked(String::from("remove")),
            parse_and_ok_mock::<StorageKeys, _>(()),
        );
        let result = set_data(&ctx, &mut host);
        claim_eq!(result, Ok(()));
    }

    #[concordium_test]
    fn test_get_data() {
        let mut host = default_host();

        let mut ctx = TestReceiveContext::empty();
        let params = GetDataParams {
            domain: String::from("test.ccd"),
            key: String::from("Twitter"),
        };
        let bytes = to_bytes(&params);
        ctx.set_sender(Address::Account(USER_1))
            .set_parameter(&bytes);
        // Get ownership info
        host.setup_mock_entrypoint(
            CNS_NFT,
            OwnedEntrypointName::new_unchecked(String::from("getTokenExpiry")),
            parse_and_ok_mock::<TokenParams, _>(Some(TokenSubscriptionStatus {
                owner: Address::Account(USER_2),
                expiry: SubscriptionExpiryStatus::Owned(
                    test_slot_time()
                        .checked_add(Duration::from_days(50))
                        .unwrap(),
                ),
            })),
        );
        // Get registry data
        host.setup_mock_entrypoint(
            REGISTRY,
            OwnedEntrypointName::new_unchecked(String::from("get")),
            parse_and_ok_mock::<StorageKeys, _>(Some(StorageGetEntryResult {
                prefix: Bytes(
                    TokenizedDomain::try_from(params.domain.as_ref())
                        .expect_report("Unable to parse domain")
                        .namehash()
                        .into(),
                ),
                entries: vec![MaybeStorageEntry {
                    key: Bytes(to_bytes(&params.key)),
                    value: Some(Bytes(to_bytes(&DataValue::Url(String::from(
                        "https://twitter.com/cns-test",
                    ))))),
                }],
            })),
        );
        let result = get_data(&ctx, &mut host);
        claim_eq!(
            result,
            Ok(DataValue::Url(
                String::from("https://twitter.com/cns-test",)
            ))
        );
    }

    #[concordium_test]
    fn test_get_missing_data() {
        let mut host = default_host();

        // Get missing key
        let mut ctx = TestReceiveContext::empty();
        let params = GetDataParams {
            domain: String::from("test.ccd"),
            key: String::from("Twitter"),
        };
        let bytes = to_bytes(&params);
        ctx.set_sender(Address::Account(USER_1))
            .set_parameter(&bytes);
        // Get ownership info
        host.setup_mock_entrypoint(
            CNS_NFT,
            OwnedEntrypointName::new_unchecked(String::from("getTokenExpiry")),
            parse_and_ok_mock::<TokenParams, _>(Some(TokenSubscriptionStatus {
                owner: Address::Account(USER_2),
                expiry: SubscriptionExpiryStatus::Owned(
                    test_slot_time()
                        .checked_add(Duration::from_days(50))
                        .unwrap(),
                ),
            })),
        );
        // Get registry data
        host.setup_mock_entrypoint(
            REGISTRY,
            OwnedEntrypointName::new_unchecked(String::from("get")),
            parse_and_ok_mock::<StorageKeys, _>(Some(StorageGetEntryResult {
                prefix: Bytes(
                    TokenizedDomain::try_from(params.domain.as_ref())
                        .expect_report("Unable to parse domain")
                        .namehash()
                        .into(),
                ),
                entries: vec![MaybeStorageEntry {
                    key: Bytes(to_bytes(&params.key)),
                    value: None,
                }],
            })),
        );
        let result = get_data(&ctx, &mut host);
        claim_eq!(result, Err(CustomContractError::NotFound.into()));
    }
    #[concordium_test]
    fn test_get_missing_subdomain() {
        let mut host = default_host();

        let mut ctx = TestReceiveContext::empty();
        let params = GetDataParams {
            domain: String::from("test.test.ccd"),
            key: String::from("Twitter"),
        };
        let bytes = to_bytes(&params);
        ctx.set_sender(Address::Account(USER_1))
            .set_parameter(&bytes);
        // Get ownership info
        host.setup_mock_entrypoint(
            CNS_NFT,
            OwnedEntrypointName::new_unchecked(String::from("getTokenExpiry")),
            parse_and_ok_mock::<TokenParams, _>(Some(TokenSubscriptionStatus {
                owner: Address::Account(USER_2),
                expiry: SubscriptionExpiryStatus::Owned(
                    test_slot_time()
                        .checked_add(Duration::from_days(50))
                        .unwrap(),
                ),
            })),
        );
        // Get registry data
        host.setup_mock_entrypoint(
            REGISTRY,
            OwnedEntrypointName::new_unchecked(String::from("get")),
            parse_and_ok_mock::<StorageKeys, _>(None::<StorageGetEntryResult>),
        );
        let result = get_data(&ctx, &mut host);
        claim_eq!(result, Err(CustomContractError::NotFound.into()));
    }

    #[concordium_test]
    fn test_create_subdomain() {
        let mut host = default_host();

        let mut ctx = TestReceiveContext::empty();
        let params = SubdomainParams {
            subdomain: String::from("test.test.ccd"),
        };
        let bytes = to_bytes(&params);
        ctx.set_parameter(&bytes)
            .set_metadata_slot_time(test_slot_time())
            .set_sender(Address::Account(USER_1))
            .set_invoker(USER_1);

        // Get ownership info
        host.setup_mock_entrypoint(
            CNS_NFT,
            OwnedEntrypointName::new_unchecked(String::from("getTokenExpiry")),
            parse_and_ok_mock::<TokenParams, _>(Some(TokenSubscriptionStatus {
                owner: Address::Account(USER_1),
                expiry: SubscriptionExpiryStatus::Owned(
                    test_slot_time()
                        .checked_add(Duration::from_days(50))
                        .unwrap(),
                ),
            })),
        );
        // Get pricing info
        host.setup_mock_entrypoint(
            PRICE_ORACLE,
            OwnedEntrypointName::new_unchecked(String::from("getYearlyDomainPrice")),
            parse_and_map_mock(test_price_oracle),
        );

        // Get registry data
        host.setup_mock_entrypoint(
            REGISTRY,
            OwnedEntrypointName::new_unchecked(String::from("insert")),
            parse_and_ok_mock::<StorageEntries, _>(()),
        );

        let invoke_amount = TEST_YEARLY_SUBDOMAIN_PRICE + Amount::from_ccd(1);
        host.set_self_balance(invoke_amount);

        let result = create_subdomain(&ctx, &mut host, invoke_amount);

        claim_eq!(result, Ok(()));
        // Transfer subscription cost
        claim!(host.transfer_occurred(&ADMIN, TEST_YEARLY_SUBDOMAIN_PRICE));
        // Return extra
        claim!(host.transfer_occurred(&USER_1, Amount::from_ccd(1)));
    }

    #[concordium_test]
    fn test_delete_subdomain() {
        let mut host = default_host();

        let mut ctx = TestReceiveContext::empty();
        let subdomain = "test.test.ccd";
        let params = SubdomainParams {
            subdomain: subdomain.into(),
        };
        let bytes = to_bytes(&params);
        ctx.set_parameter(&bytes)
            .set_metadata_slot_time(test_slot_time())
            .set_sender(Address::Account(USER_1));

        // Get ownership info
        host.setup_mock_entrypoint(
            CNS_NFT,
            OwnedEntrypointName::new_unchecked(String::from("getTokenExpiry")),
            parse_and_ok_mock::<TokenParams, _>(Some(TokenSubscriptionStatus {
                owner: Address::Account(USER_1),
                expiry: SubscriptionExpiryStatus::Owned(
                    test_slot_time()
                        .checked_add(Duration::from_days(50))
                        .unwrap(),
                ),
            })),
        );
        // Clear registry data
        host.setup_mock_entrypoint(
            REGISTRY,
            OwnedEntrypointName::new_unchecked(String::from("remove")),
            parse_and_check_mock::<StorageKeys, _>(
                move |params| {
                    // Ensure subdomain prefix
                    params.prefix
                        == Bytes(
                            TokenizedDomain::try_from(subdomain)
                                .expect_report("Invalid domain")
                                .namehash()
                                .into(),
                        )
                        // Ensure all keys are cleared
                        && params.keys == StorageKeySelection::All
                },
                (),
            ),
        );

        let result = delete_subdomain(&ctx, &mut host);

        claim_eq!(result, Ok(()));
    }
}
