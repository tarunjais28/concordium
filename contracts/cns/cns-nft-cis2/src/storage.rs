use commons::*;
use concordium_std::*;

use crate::state::State;

const OWNER_KEY: &str = "owner";
const EXPIRY_KEY: &str = "expiry";
const GRACE_KEY: &str = "grace";
const DOMAIN_KEY: &str = "domain";
const ROYALTY_KEY: &str = "royalty";

pub struct SubscriptionData {
    pub owner: Address,
    pub expiry: Timestamp,
    pub grace: Duration,
}

impl SubscriptionData {
    pub fn into_status(self, slot_time: Timestamp) -> commons::TokenSubscriptionStatus {
        let grace_period = self.expiry.checked_add(self.grace).unwrap();

        let expiry = if self.expiry >= slot_time {
            SubscriptionExpiryStatus::Owned(self.expiry)
        } else if grace_period >= slot_time {
            SubscriptionExpiryStatus::Grace(grace_period)
        } else {
            SubscriptionExpiryStatus::Expired
        };

        TokenSubscriptionStatus {
            owner: self.owner,
            expiry,
        }
    }
}

pub struct TokenData {
    pub owner: Address,
    pub expiry: Timestamp,
    pub grace: Duration,
    pub domain: String,
    pub royalty: Percentage,
}

/// Query the storage contract for token subscription data
pub fn get_token_subscription_data<S: HasStateApi>(
    host: &impl HasHost<State<S>>,
    storage_addr: &ContractAddress,
    token_id: &ContractTokenId,
) -> ContractResult<Option<SubscriptionData>> {
    if let Some(data) = host
        .storage_get_raw(
            storage_addr,
            &StorageKeysRef::some(
                token_id.0.as_slice().into(),
                &[OWNER_KEY.as_ref(), EXPIRY_KEY.as_ref(), GRACE_KEY.as_ref()],
            ),
        )
        .map_err(handle_storage_get_error)?
    {
        let owner = get_entry_cursor(OWNER_KEY, &data)
            .and_then(|mut cursor| Deserial::deserial(&mut cursor).ok())
            .ok_or(CustomContractError::InvalidFields)?;
        let expiry = get_entry_cursor(EXPIRY_KEY, &data)
            .and_then(|mut cursor| Deserial::deserial(&mut cursor).ok())
            .ok_or(CustomContractError::InvalidFields)?;
        let grace = get_entry_cursor(GRACE_KEY, &data)
            .and_then(|mut cursor| Deserial::deserial(&mut cursor).ok())
            .ok_or(CustomContractError::InvalidFields)?;

        Ok(Some(SubscriptionData {
            owner,
            expiry,
            grace,
        }))
    } else {
        Ok(None)
    }
}

/// Query the storage contract for token royalty
pub fn get_token_royalty<S: HasStateApi>(
    host: &impl HasHost<State<S>>,
    storage_addr: &ContractAddress,
    token_id: &ContractTokenId,
) -> ContractResult<Option<Percentage>> {
    host.storage_get_raw(
        storage_addr,
        &StorageKeysRef::some(token_id.0.as_slice().into(), &[ROYALTY_KEY.as_ref()]),
    )
    .map_err(handle_storage_get_error)?
    .map(|data| {
        get_entry_cursor(ROYALTY_KEY, &data)
            .and_then(|mut cursor| Deserial::deserial(&mut cursor).ok())
            .ok_or(ContractError::Custom(CustomContractError::InvalidFields))
    })
    .transpose()
}

/// Query the storage contract for token info
pub fn get_token_info<S: HasStateApi>(
    host: &impl HasHost<State<S>>,
    storage_addr: &ContractAddress,
    token_id: &ContractTokenId,
) -> ContractResult<Option<TokenInfo>> {
    if let Some(data) = host
        .storage_get_raw(
            storage_addr,
            &StorageKeysRef::some(
                token_id.0.as_slice().into(),
                &[DOMAIN_KEY.as_ref(), ROYALTY_KEY.as_ref()],
            ),
        )
        .map_err(handle_storage_get_error)?
    {
        let domain = get_entry_cursor(DOMAIN_KEY, &data)
            .and_then(|mut cursor| Deserial::deserial(&mut cursor).ok())
            .ok_or(CustomContractError::InvalidFields)?;
        let royalty = get_entry_cursor(ROYALTY_KEY, &data)
            .and_then(|mut cursor| Deserial::deserial(&mut cursor).ok())
            .ok_or(CustomContractError::InvalidFields)?;

        Ok(Some(TokenInfo { domain, royalty }))
    } else {
        Ok(None)
    }
}

/// Query the storage contract for expiry data
pub fn get_expiry<S: HasStateApi>(
    host: &impl HasHost<State<S>>,
    storage_addr: &ContractAddress,
    token_id: &ContractTokenId,
) -> ContractResult<Option<Timestamp>> {
    host.storage_get_raw(
        storage_addr,
        &StorageKeysRef::some(token_id.0.as_slice().into(), &[EXPIRY_KEY.as_ref()]),
    )
    .map_err(handle_storage_get_error)?
    .map(|data| {
        get_entry_cursor(EXPIRY_KEY, &data)
            .and_then(|mut cursor| Deserial::deserial(&mut cursor).ok())
            .ok_or(ContractError::Custom(CustomContractError::InvalidFields))
    })
    .transpose()
}

pub fn insert_token<S: HasStateApi>(
    host: &mut impl HasHost<State<S>>,
    storage_addr: &ContractAddress,
    token_id: &ContractTokenId,
    token_data: TokenData,
) -> ContractResult<()> {
    host.storage_insert_raw(
        storage_addr,
        &StorageEntriesRef {
            prefix: token_id.0.as_slice().into(),
            entries: &[
                StorageEntryRef::new(
                    OWNER_KEY.as_ref(),
                    Bytes(to_bytes(&token_data.owner)).as_ref(),
                ),
                StorageEntryRef::new(
                    EXPIRY_KEY.as_ref(),
                    Bytes(to_bytes(&token_data.expiry)).as_ref(),
                ),
                StorageEntryRef::new(
                    GRACE_KEY.as_ref(),
                    Bytes(to_bytes(&token_data.grace)).as_ref(),
                ),
                StorageEntryRef::new(
                    DOMAIN_KEY.as_ref(),
                    Bytes(to_bytes(&token_data.domain)).as_ref(),
                ),
                StorageEntryRef::new(
                    ROYALTY_KEY.as_ref(),
                    Bytes(to_bytes(&token_data.royalty)).as_ref(),
                ),
            ],
        },
    )
    .map_err(handle_call_error)
}

pub fn update_expiry<S: HasStateApi>(
    host: &mut impl HasHost<State<S>>,
    storage_addr: &ContractAddress,
    token_id: &ContractTokenId,
    expiry: Timestamp,
) -> ContractResult<()> {
    host.storage_update_raw(
        storage_addr,
        &StorageEntriesRef {
            prefix: token_id.0.as_slice().into(),
            entries: &[StorageEntryRef::new(
                EXPIRY_KEY.as_ref(),
                Bytes(to_bytes(&expiry)).as_ref(),
            )],
        },
    )
    .map_err(handle_call_error)
}

pub fn update_owner<S: HasStateApi>(
    host: &mut impl HasHost<State<S>>,
    storage_addr: &ContractAddress,
    token_id: &ContractTokenId,
    owner: Address,
) -> ContractResult<()> {
    host.storage_update_raw(
        storage_addr,
        &StorageEntriesRef {
            prefix: token_id.0.as_slice().into(),
            entries: &[StorageEntryRef::new(
                OWNER_KEY.as_ref(),
                Bytes(to_bytes(&owner)).as_ref(),
            )],
        },
    )
    .map_err(handle_call_error)
}

#[inline(never)]
fn get_entry_cursor<'r>(
    key: &str,
    result: &'r StorageGetEntryResult,
) -> Option<Cursor<&'r Vec<u8>>> {
    result
        .entries
        .iter()
        .find(|entry| entry.key.as_ref() == AsRef::<ByteSlice>::as_ref(key))
        .map(|maybe_entry| maybe_entry.value.as_ref())
        .flatten()
        .map(|bytes| Cursor::new(&bytes.0))
}

fn handle_call_error<T>(error: CallContractError<T>) -> ContractError {
    match error {
        CallContractError::MissingEntrypoint => CustomContractError::Incompatible.into(),
        CallContractError::LogicReject { reason, .. } => match reason {
            // CustomContractError::ParseParams | Concordium ParseError
            -1 | -2147483646 => CustomContractError::Incompatible.into(),
            // CustomContractError::NotFound
            -30 => ContractError::InvalidTokenId,
            // CustomContractError::AlreadyExists
            -35 => ContractError::Unauthorized,
            // Remaining errors
            _ => CustomContractError::InvokeContractError.into(),
        },
        e => e.into(),
    }
}

fn handle_storage_get_error<T>(error: ContractReadError<T>) -> ContractError {
    match error {
        ContractReadError::Call(e) => handle_call_error(e),
        _ => ContractError::Custom(CustomContractError::Incompatible),
    }
}
