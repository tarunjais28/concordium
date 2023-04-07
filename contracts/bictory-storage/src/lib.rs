#![no_std]

use commons::{
    AuthorityUpdateParams, AuthorityViewParams, CustomContractError, MaybeStorageEntry,
    StorageEntries, StorageGetEntryResult, StorageKeySelection, StorageKeys,
};
use concordium_std::*;

mod state;

use state::State;

#[init(contract = "BictoryStorage")]
fn init<S: HasStateApi>(
    ctx: &impl HasInitContext,
    state_builder: &mut StateBuilder<S>,
) -> InitResult<State<S>> {
    Ok(State::new(state_builder, ctx.init_origin()))
}

#[receive(
    mutable,
    contract = "BictoryStorage",
    name = "insert",
    parameter = "StorageEntries"
)]
fn insert<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ReceiveResult<()> {
    let (state, builder) = host.state_and_builder();

    ensure!(
        state.has_writer_rights(&ctx.sender()),
        CustomContractError::Unauthorized.into()
    );

    let params = StorageEntries::deserial(&mut ctx.parameter_cursor())?;

    match state.storage.entry(params.prefix) {
        Entry::Vacant(hole) => {
            let mut map = builder.new_map();
            for entry in params.entries {
                map.insert(entry.key, entry.value);
            }
            hole.insert(map);

            Ok(())
        }
        Entry::Occupied(_) => Err(CustomContractError::AlreadyExists.into()),
    }
}

#[receive(
    mutable,
    contract = "BictoryStorage",
    name = "update",
    parameter = "StorageEntries"
)]
fn update<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ReceiveResult<()> {
    let state = host.state_mut();

    ensure!(
        state.has_writer_rights(&ctx.sender()),
        CustomContractError::Unauthorized.into()
    );

    let params = StorageEntries::deserial(&mut ctx.parameter_cursor())?;

    match state.storage.entry(params.prefix) {
        Entry::Occupied(mut map) => {
            for entry in params.entries {
                map.insert(entry.key, entry.value);
            }
            Ok(())
        }
        Entry::Vacant(_) => Err(CustomContractError::NotFound.into()),
    }
}

#[receive(
    mutable,
    contract = "BictoryStorage",
    name = "remove",
    parameter = "StorageKeys"
)]
fn remove<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ReceiveResult<()> {
    let state = host.state_mut();

    ensure!(
        state.has_writer_rights(&ctx.sender()),
        CustomContractError::Unauthorized.into()
    );

    let params = StorageKeys::deserial(&mut ctx.parameter_cursor())?;
    match params.keys {
        StorageKeySelection::All => state
            .storage
            .remove_and_get(&params.prefix)
            .map(|v| v.delete())
            .ok_or(CustomContractError::NotFound)?,
        StorageKeySelection::Some(key_list) => match state.storage.get_mut(&params.prefix) {
            Some(mut map) => {
                for key in key_list {
                    map.remove(&key);
                }
            }
            None => Err(CustomContractError::NotFound)?,
        },
    }

    Ok(())
}

#[receive(
    contract = "BictoryStorage",
    name = "get",
    parameter = "StorageKeys",
    return_value = "Option<StorageGetEntryResult>"
)]
fn get<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &impl HasHost<State<S>, StateApiType = S>,
) -> ReceiveResult<Option<StorageGetEntryResult>> {
    let state = host.state();
    let params = StorageKeys::deserial(&mut ctx.parameter_cursor())?;

    let result = state.storage.get(&params.prefix).map(|map| {
        let entries = match params.keys {
            StorageKeySelection::All => map.iter().fold(vec![], |mut acc, (key, value)| {
                acc.push(MaybeStorageEntry {
                    key: key.clone(),
                    value: Some(value.clone()),
                });
                acc
            }),
            StorageKeySelection::Some(key_list) => key_list
                .into_iter()
                .map(|key| {
                    let value = map.get(&key);
                    MaybeStorageEntry {
                        key,
                        value: value.map(|v| v.clone()),
                    }
                })
                .collect(),
        };
        StorageGetEntryResult {
            prefix: params.prefix,
            entries,
        }
    });

    Ok(result)
}

#[receive(
    mutable,
    contract = "BictoryStorage",
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

#[receive(
    contract = "BictoryStorage",
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

#[derive(Debug, SchemaType, Serialize)]
enum UpdateKind {
    Remove,
    Add,
}

#[derive(Debug, SchemaType, Serialize)]
struct UpdateWriterParams {
    kind: UpdateKind,
    address: Address,
}

#[receive(
    mutable,
    contract = "BictoryStorage",
    name = "updateWriter",
    parameter = "UpdateWriterParams"
)]
fn update_writer<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ReceiveResult<()> {
    let state = host.state_mut();
    let params = UpdateWriterParams::deserial(&mut ctx.parameter_cursor())?;

    ensure!(
        state.authority.has_maintainer_rights(&ctx.sender()),
        CustomContractError::Unauthorized.into()
    );

    match params.kind {
        UpdateKind::Remove => {
            state.writers.remove(&params.address);
        }
        UpdateKind::Add => {
            state.writers.insert(params.address);
        }
    }

    Ok(())
}

#[derive(Debug, SchemaType, Serialize)]
struct ViewWritersParams {
    skip: u32,
    show: u32,
}

#[receive(
    contract = "BictoryStorage",
    name = "viewWriters",
    parameter = "ViewWritersParams",
    return_value = "Vec<Address>"
)]
fn view_writers<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &impl HasHost<State<S>, StateApiType = S>,
) -> ReceiveResult<Vec<Address>> {
    let params = ViewWritersParams::deserial(&mut ctx.parameter_cursor())?;

    let result = host
        .state()
        .writers
        .iter()
        .skip(params.skip as usize)
        .take(params.show as usize)
        .map(|a| *a)
        .collect();

    Ok(result)
}

#[concordium_cfg_test]
mod tests {
    use super::*;
    use commons::{AuthorityField, AuthorityUpdateKind, Bytes, StorageEntry};
    use concordium_std::test_infrastructure::*;

    const AUTHORIZED_CALLER: ContractAddress = ContractAddress {
        index: 1,
        subindex: 0,
    };

    const UNAUTHORIZED_CALLER: ContractAddress = ContractAddress {
        index: 1,
        subindex: 1,
    };

    const ADMIN: AccountAddress = AccountAddress([1; 32]);
    const MAINTAINER: AccountAddress = AccountAddress([2; 32]);

    fn default_host() -> TestHost<State<TestStateApi>> {
        let mut ctx = TestInitContext::empty();
        // admin is initialized to `ctx.origin()`
        ctx.set_init_origin(ADMIN);
        let mut state_builder = TestStateBuilder::new();

        // Call the init method.
        let state =
            init(&ctx, &mut state_builder).expect_report("Failed during init_BictoryStorage");

        let mut host = TestHost::new(state, state_builder);

        let mut ctx = TestReceiveContext::default();
        let bytes = to_bytes(&AuthorityUpdateParams {
            field: AuthorityField::Maintainer,
            kind: AuthorityUpdateKind::Add,
            address: Address::Account(MAINTAINER),
        });
        ctx.set_sender(Address::Account(ADMIN))
            .set_parameter(&bytes);
        let result = update_authority(&ctx, &mut host);
        claim!(result.is_ok());

        let mut ctx = TestReceiveContext::default();
        let bytes = to_bytes(&UpdateWriterParams {
            kind: UpdateKind::Add,
            address: Address::Contract(AUTHORIZED_CALLER),
        });
        ctx.set_sender(Address::Account(MAINTAINER))
            .set_parameter(&bytes);
        let result = update_writer(&ctx, &mut host);
        claim!(result.is_ok());

        host
    }

    #[concordium_test]
    fn test_init_test_state() {
        let host = default_host();
        let state = host.state();

        // Assert properties.
        claim!(state.authority.has_admin_rights(&Address::Account(ADMIN)));
        claim!(!state
            .authority
            .has_admin_rights(&Address::Account(MAINTAINER)));
        claim!(!state
            .authority
            .has_admin_rights(&Address::Contract(AUTHORIZED_CALLER)));

        claim!(state
            .authority
            .has_maintainer_rights(&Address::Account(ADMIN)));
        claim!(state
            .authority
            .has_maintainer_rights(&Address::Account(MAINTAINER)));
        claim!(!state
            .authority
            .has_maintainer_rights(&Address::Contract(AUTHORIZED_CALLER)));

        claim!(!state.writers.contains(&Address::Account(ADMIN)));
        claim!(!state.writers.contains(&Address::Account(MAINTAINER)));
        claim!(state
            .writers
            .contains(&Address::Contract(AUTHORIZED_CALLER)));

        claim!(state.storage.is_empty());
    }

    #[concordium_test]
    fn test_insert() {
        let mut host = default_host();

        // Authorized new insert
        let params = to_bytes(&StorageEntries {
            prefix: Bytes::from([0, 0]),
            entries: vec![StorageEntry {
                key: Bytes::from([1, 2, 3]),
                value: Bytes::from([4, 5, 6]),
            }],
        });
        let mut ctx = TestReceiveContext::default();
        ctx.set_parameter(&params)
            .set_sender(Address::Contract(AUTHORIZED_CALLER));

        let result = insert(&ctx, &mut host);
        claim_eq!(result, Ok(()));
        claim_eq!(
            host.state()
                .storage
                .get(&Bytes::from([0, 0]))
                .and_then(|map| map.get(&Bytes::from([1, 2, 3])).map(|v| v.clone())),
            Some(Bytes::from([4, 5, 6]))
        );

        // Unauthorized new insert
        let params = to_bytes(&StorageEntries {
            prefix: Bytes::from([1, 1]),
            entries: vec![StorageEntry {
                key: Bytes::from([2, 3, 4]),
                value: Bytes::from([5, 6, 7]),
            }],
        });
        let mut ctx = TestReceiveContext::default();
        ctx.set_parameter(&params)
            .set_sender(Address::Contract(UNAUTHORIZED_CALLER));

        let result = insert(&ctx, &mut host);
        claim_eq!(result, Err(CustomContractError::Unauthorized.into()));

        // Authorized duplicate insert
        let params = to_bytes(&StorageEntries {
            prefix: Bytes::from([0, 0]),
            entries: vec![StorageEntry {
                key: Bytes::from([1, 2, 3]),
                value: Bytes::from([4, 5, 6]),
            }],
        });
        let mut ctx = TestReceiveContext::default();
        ctx.set_parameter(&params)
            .set_sender(Address::Contract(AUTHORIZED_CALLER));

        let result = insert(&ctx, &mut host);
        claim_eq!(result, Err(CustomContractError::AlreadyExists.into()));
    }

    #[concordium_test]
    fn test_update() {
        let mut host = default_host();

        // Authorized missing update
        let params = to_bytes(&StorageEntries {
            prefix: Bytes::from([0, 0]),
            entries: vec![StorageEntry {
                key: Bytes::from([1, 2, 3]),
                value: Bytes::from([9, 8, 7]),
            }],
        });
        let mut ctx = TestReceiveContext::default();
        ctx.set_parameter(&params)
            .set_sender(Address::Contract(AUTHORIZED_CALLER));

        let result = update(&ctx, &mut host);
        claim_eq!(result, Err(CustomContractError::NotFound.into()));

        // Insert
        let params = to_bytes(&StorageEntries {
            prefix: Bytes::from([0, 0]),
            entries: vec![StorageEntry {
                key: Bytes::from([1, 2, 3]),
                value: Bytes::from([4, 5, 6]),
            }],
        });
        let mut ctx = TestReceiveContext::default();
        ctx.set_parameter(&params)
            .set_sender(Address::Contract(AUTHORIZED_CALLER));
        claim_eq!(insert(&ctx, &mut host), Ok(()));

        // Authorized existing update
        let params = to_bytes(&StorageEntries {
            prefix: Bytes::from([0, 0]),
            entries: vec![StorageEntry {
                key: Bytes::from([1, 2, 3]),
                value: Bytes::from([9, 8, 7]),
            }],
        });
        let mut ctx = TestReceiveContext::default();
        ctx.set_parameter(&params)
            .set_sender(Address::Contract(AUTHORIZED_CALLER));

        let result = update(&ctx, &mut host);
        claim_eq!(result, Ok(()));
        claim_eq!(
            host.state()
                .storage
                .get(&Bytes::from([0, 0]))
                .and_then(|map| map.get(&Bytes::from([1, 2, 3])).map(|v| v.clone())),
            Some(Bytes::from([9, 8, 7]))
        );

        // Unauthorized existing update
        let params = to_bytes(&StorageEntries {
            prefix: Bytes::from([0, 0]),
            entries: vec![StorageEntry {
                key: Bytes::from([1, 2, 3]),
                value: Bytes::from([4, 5, 6]),
            }],
        });
        let mut ctx = TestReceiveContext::default();
        ctx.set_parameter(&params)
            .set_sender(Address::Contract(UNAUTHORIZED_CALLER));

        let result = update(&ctx, &mut host);
        claim_eq!(result, Err(CustomContractError::Unauthorized.into()));
    }

    #[concordium_test]
    fn test_remove() {
        let mut host = default_host();

        // Authorized missing prefix remove
        let params = to_bytes(&StorageKeys {
            prefix: Bytes::from([0, 0]),
            keys: StorageKeySelection::All,
        });
        let mut ctx = TestReceiveContext::default();
        ctx.set_parameter(&params)
            .set_sender(Address::Contract(AUTHORIZED_CALLER));

        let result = remove(&ctx, &mut host);
        claim_eq!(result, Err(CustomContractError::NotFound.into()));

        // Authorized missing prefix remove
        let params = to_bytes(&StorageKeys {
            prefix: Bytes::from([0, 0]),
            keys: StorageKeySelection::Some(vec![Bytes::from([1, 2, 3])]),
        });
        let mut ctx = TestReceiveContext::default();
        ctx.set_parameter(&params)
            .set_sender(Address::Contract(AUTHORIZED_CALLER));

        let result = remove(&ctx, &mut host);
        claim_eq!(result, Err(CustomContractError::NotFound.into()));

        // Unauthorized missing remove
        let params = to_bytes(&StorageKeys {
            prefix: Bytes::from([0, 0]),
            keys: StorageKeySelection::All,
        });
        let mut ctx = TestReceiveContext::default();
        ctx.set_parameter(&params)
            .set_sender(Address::Contract(UNAUTHORIZED_CALLER));

        let result = remove(&ctx, &mut host);
        claim_eq!(result, Err(CustomContractError::Unauthorized.into()));

        // Insert
        let params = to_bytes(&StorageEntries {
            prefix: Bytes::from([0, 0]),
            entries: vec![
                StorageEntry {
                    key: Bytes::from([1, 2, 3]),
                    value: Bytes::from([9, 9, 9]),
                },
                StorageEntry {
                    key: Bytes::from([4, 5, 6]),
                    value: Bytes::from([8, 8, 8]),
                },
                StorageEntry {
                    key: Bytes::from([7, 8, 9]),
                    value: Bytes::from([7, 7, 7]),
                },
            ],
        });
        let mut ctx = TestReceiveContext::default();
        ctx.set_parameter(&params)
            .set_sender(Address::Contract(AUTHORIZED_CALLER));
        claim_eq!(insert(&ctx, &mut host), Ok(()));

        // Unauthorized existing remove
        let params = to_bytes(&StorageKeys {
            prefix: Bytes::from([0, 0]),
            keys: StorageKeySelection::Some(vec![Bytes::from([1, 2, 3])]),
        });
        let mut ctx = TestReceiveContext::default();
        ctx.set_parameter(&params)
            .set_sender(Address::Contract(UNAUTHORIZED_CALLER));

        let result = remove(&ctx, &mut host);
        claim_eq!(result, Err(CustomContractError::Unauthorized.into()));

        // Authorized existing partial remove
        let params = to_bytes(&StorageKeys {
            prefix: Bytes::from([0, 0]),
            keys: StorageKeySelection::Some(vec![Bytes::from([1, 2, 3])]),
        });
        let mut ctx = TestReceiveContext::default();
        ctx.set_parameter(&params)
            .set_sender(Address::Contract(AUTHORIZED_CALLER));

        let result = remove(&ctx, &mut host);
        claim_eq!(result, Ok(()));
        claim_eq!(
            host.state()
                .storage
                .get(&Bytes::from([0, 0]))
                .and_then(|map| map.get(&Bytes::from([1, 2, 3])).map(|v| v.clone())),
            None
        );
        claim_eq!(
            host.state()
                .storage
                .get(&Bytes::from([0, 0]))
                .and_then(|map| map.get(&Bytes::from([4, 5, 6])).map(|v| v.clone())),
            Some(Bytes::from([8, 8, 8]))
        );
        claim_eq!(
            host.state()
                .storage
                .get(&Bytes::from([0, 0]))
                .and_then(|map| map.get(&Bytes::from([7, 8, 9])).map(|v| v.clone())),
            Some(Bytes::from([7, 7, 7]))
        );

        // Authorized existing prefix remove
        let params = to_bytes(&StorageKeys {
            prefix: Bytes::from([0, 0]),
            keys: StorageKeySelection::All,
        });
        let mut ctx = TestReceiveContext::default();
        ctx.set_parameter(&params)
            .set_sender(Address::Contract(AUTHORIZED_CALLER));

        let result = remove(&ctx, &mut host);
        claim_eq!(result, Ok(()));
        claim!(host.state().storage.get(&Bytes::from([0, 0])).is_none());
    }

    #[concordium_test]
    fn test_get() {
        let mut host = default_host();

        // Get missing prefix
        let params = to_bytes(&StorageKeys {
            prefix: Bytes::from([0, 0]),
            keys: StorageKeySelection::Some(vec![Bytes::from([1, 2, 3])]),
        });
        let mut ctx = TestReceiveContext::default();
        ctx.set_parameter(&params);

        let result = get(&ctx, &mut host);
        claim_eq!(result, Ok(None));

        // Insert
        let params = to_bytes(&StorageEntries {
            prefix: Bytes::from([0, 0]),
            entries: vec![
                StorageEntry {
                    key: Bytes::from([1, 2, 3]),
                    value: Bytes::from([9, 9, 9]),
                },
                StorageEntry {
                    key: Bytes::from([4, 5, 6]),
                    value: Bytes::from([8, 8, 8]),
                },
                StorageEntry {
                    key: Bytes::from([7, 8, 9]),
                    value: Bytes::from([7, 7, 7]),
                },
            ],
        });
        let mut ctx = TestReceiveContext::default();
        ctx.set_parameter(&params)
            .set_sender(Address::Contract(AUTHORIZED_CALLER));
        claim_eq!(insert(&ctx, &mut host), Ok(()));

        // Get missing key
        let params = to_bytes(&StorageKeys {
            prefix: Bytes::from([0, 0]),
            keys: StorageKeySelection::Some(vec![Bytes::from([10, 11, 12])]),
        });
        let mut ctx = TestReceiveContext::default();
        ctx.set_parameter(&params);

        let result = get(&ctx, &mut host);
        claim_eq!(
            result,
            Ok(Some(StorageGetEntryResult {
                prefix: Bytes::from([0, 0]),
                entries: vec![MaybeStorageEntry {
                    key: Bytes::from([10, 11, 12]),
                    value: None,
                }]
            }))
        );

        // Get some existing
        let params = to_bytes(&StorageKeys {
            prefix: Bytes::from([0, 0]),
            keys: StorageKeySelection::Some(vec![Bytes::from([1, 2, 3])]),
        });
        let mut ctx = TestReceiveContext::default();
        ctx.set_parameter(&params);

        let result = get(&ctx, &mut host);
        claim_eq!(
            result,
            Ok(Some(StorageGetEntryResult {
                prefix: Bytes::from([0, 0]),
                entries: vec![MaybeStorageEntry {
                    key: Bytes::from([1, 2, 3]),
                    value: Some(Bytes::from([9, 9, 9])),
                }]
            }))
        );

        // Get all existing
        let params = to_bytes(&StorageKeys {
            prefix: Bytes::from([0, 0]),
            keys: StorageKeySelection::All,
        });
        let mut ctx = TestReceiveContext::default();
        ctx.set_parameter(&params);

        let result = get(&ctx, &mut host);
        claim_eq!(
            result,
            Ok(Some(StorageGetEntryResult {
                prefix: Bytes::from([0, 0]),
                entries: vec![
                    MaybeStorageEntry {
                        key: Bytes::from([1, 2, 3]),
                        value: Some(Bytes::from([9, 9, 9])),
                    },
                    MaybeStorageEntry {
                        key: Bytes::from([4, 5, 6]),
                        value: Some(Bytes::from([8, 8, 8])),
                    },
                    MaybeStorageEntry {
                        key: Bytes::from([7, 8, 9]),
                        value: Some(Bytes::from([7, 7, 7])),
                    }
                ]
            }))
        );
    }
}
