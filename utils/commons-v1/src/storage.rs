use super::*;

pub trait HostStorageExt<S>: HasHost<S> {
    fn storage_insert<K: Serial, V: Serial>(
        &mut self,
        contract: &ContractAddress,
        prefix: &ByteSlice,
        key: &K,
        value: &V,
    ) -> Result<(), CallContractError<Self::ReturnValueType>> {
        let params = serial_single_entry(prefix, key, value);

        self.invoke_contract_raw(
            contract,
            Parameter(params.as_slice()),
            EntrypointName::new_unchecked("insert"),
            Amount::zero(),
        )?;

        Ok(())
    }

    fn storage_insert_raw(
        &mut self,
        contract: &ContractAddress,
        entries: &StorageEntriesRef,
    ) -> Result<(), CallContractError<Self::ReturnValueType>> {
        self.invoke_contract(
            contract,
            entries,
            EntrypointName::new_unchecked("insert"),
            Amount::zero(),
        )?;
        Ok(())
    }

    fn storage_update<K: Serial, V: Serial>(
        &mut self,
        contract: &ContractAddress,
        prefix: &ByteSlice,
        key: &K,
        value: &V,
    ) -> Result<(), CallContractError<Self::ReturnValueType>> {
        let params = serial_single_entry(prefix, key, value);

        self.invoke_contract_raw(
            contract,
            Parameter(params.as_slice()),
            EntrypointName::new_unchecked("update"),
            Amount::zero(),
        )?;

        Ok(())
    }

    fn storage_update_raw(
        &mut self,
        contract: &ContractAddress,
        entries: &StorageEntriesRef,
    ) -> Result<(), CallContractError<Self::ReturnValueType>> {
        self.invoke_contract(
            contract,
            entries,
            EntrypointName::new_unchecked("update"),
            Amount::zero(),
        )?;
        Ok(())
    }

    fn storage_remove<K: Serial>(
        &mut self,
        contract: &ContractAddress,
        prefix: &ByteSlice,
        key: &K,
    ) -> Result<(), CallContractError<Self::ReturnValueType>> {
        let params = serial_single_key(prefix, key);

        self.invoke_contract_raw(
            contract,
            Parameter(params.as_slice()),
            EntrypointName::new_unchecked("remove"),
            Amount::zero(),
        )?;

        Ok(())
    }

    fn storage_remove_raw(
        &mut self,
        contract: &ContractAddress,
        entries: &StorageKeysRef,
    ) -> Result<(), CallContractError<Self::ReturnValueType>> {
        self.invoke_contract(
            contract,
            entries,
            EntrypointName::new_unchecked("remove"),
            Amount::zero(),
        )?;
        Ok(())
    }

    fn storage_get<K: Serial, R: Deserial>(
        &self,
        contract: &ContractAddress,
        prefix: &ByteSlice,
        key: &K,
    ) -> Result<Option<R>, ContractReadError<Self::ReturnValueType>> {
        let params = serial_single_key(prefix, key);

        let response = match self.invoke_contract_raw_read_only(
            contract,
            Parameter(params.as_slice()),
            EntrypointName::new_unchecked("get"),
            Amount::zero(),
        ) {
            Ok(Some(mut bytes)) => <Option<StorageGetEntryResult>>::deserial(&mut bytes)
                .map_err(|_| ContractReadError::Compatibility),
            Ok(None) => Err(ContractReadError::Compatibility),
            Err(e) => Err(ContractReadError::Call(e)),
        }?;

        response
            .and_then(|mut r| r.entries.pop())
            .and_then(|maybe_entry| maybe_entry.value)
            .map(|bytes| from_bytes(&bytes.0))
            .transpose()
            .map_err(|_| ContractReadError::Parse)
    }

    fn storage_get_raw(
        &self,
        contract: &ContractAddress,
        entries: &StorageKeysRef,
    ) -> Result<Option<StorageGetEntryResult>, ContractReadError<Self::ReturnValueType>> {
        match self.invoke_contract_read_only(
            contract,
            entries,
            EntrypointName::new_unchecked("get"),
            Amount::zero(),
        ) {
            Ok(Some(mut val)) => <Option<StorageGetEntryResult>>::deserial(&mut val)
                .map_err(|_| ContractReadError::Compatibility),
            Ok(None) => Err(ContractReadError::Compatibility),
            Err(e) => Err(ContractReadError::Call(e)),
        }
    }
}

pub(crate) fn serial_single_entry<K: Serial, V: Serial>(
    prefix: &ByteSlice,
    key: &K,
    value: &V,
) -> Vec<u8> {
    let mut buffer = Vec::new();
    let mut cursor = Cursor::new(&mut buffer);

    // Write prefix
    prefix
        .serial(&mut cursor)
        .expect("Writing to vec shouldn't fail");

    // Write entries len
    cursor.write_u16(1).expect("Writing to vec shouldn't fail");

    // Serialize key K first, and then serialize result as Bytes
    let key_offset = cursor.offset;
    cursor.write_u16(0).expect("Writing to vec shouldn't fail");
    key.serial(&mut cursor)
        .expect("Writing to vec shouldn't fail");
    let key_size = cursor.offset - key_offset - 2;
    cursor.offset = key_offset;
    cursor
        .write_u16(key_size as u16)
        .expect("Writing to vec shouldn't fail");
    let value_offset = cursor.offset + key_size;

    // Serialize value V first, and then serialize result as Bytes
    cursor.offset = value_offset;
    cursor
        .write_u16(key_size as u16)
        .expect("Writing to vec shouldn't fail");
    value
        .serial(&mut cursor)
        .expect("Writing to vec shouldn't fail");
    let value_size = cursor.offset - value_offset - 2;
    cursor.offset = value_offset;
    cursor
        .write_u16(value_size as u16)
        .expect("Writing to vec shouldn't fail");

    buffer
}

pub(crate) fn serial_single_key<K: Serial>(prefix: &ByteSlice, key: &K) -> Vec<u8> {
    let mut buffer = Vec::new();
    let mut cursor = Cursor::new(&mut buffer);

    // Write prefix
    prefix
        .serial(&mut cursor)
        .expect("Writing to vec shouldn't fail");

    // Write key selection tag
    cursor.write_u8(1).expect("Writing to vec shouldn't fail");

    // Write keys len
    cursor.write_u16(1).expect("Writing to vec shouldn't fail");

    // Serialize key K first, and then prepend length to the result
    let key_offset = cursor.offset;
    cursor.write_u16(0).expect("Writing to vec shouldn't fail");
    key.serial(&mut cursor)
        .expect("Writing to vec shouldn't fail");
    let key_size = cursor.offset - key_offset - 2;
    cursor.offset = key_offset;
    cursor
        .write_u16(key_size as u16)
        .expect("Writing to vec shouldn't fail");

    buffer
}

impl<S, H: HasHost<S>> HostStorageExt<S> for H {}

#[concordium_cfg_test]
mod tests {
    use super::*;
    use crate::{Bytes, StorageEntry};
    use test_infrastructure::*;

    const STORAGE_ADDR: ContractAddress = ContractAddress {
        index: 1,
        subindex: 1,
    };

    #[derive(Serialize)]
    struct TestState();

    fn parse_and_ok_mock<D: Deserial, R: Clone + Serial + 'static>(
        return_value: R,
    ) -> MockFn<TestState> {
        MockFn::new(
            move |parameter, _amount, _balance, _state| -> CallContractResult<R> {
                D::deserial(&mut Cursor::new(parameter)).map_err(|_| {
                    CallContractError::LogicReject {
                        reason: Reject::from(ParseError {}).error_code.into(),
                        return_value: return_value.clone(),
                    }
                })?;
                Ok((false, Some(return_value.clone())))
            },
        )
    }

    #[concordium_test]
    fn test_serialize_single_entry() {
        let prefix = Bytes(String::from("my_prefix").into_bytes());
        let key = String::from("my_key");
        let value = Address::Account(AccountAddress([2; 32]));

        let serialized = serial_single_entry(prefix.as_ref(), &key, &value);
        let control = to_bytes(&StorageEntries {
            prefix,
            entries: vec![StorageEntry {
                key: Bytes(to_bytes(&key)),
                value: Bytes(to_bytes(&value)),
            }],
        });

        claim_eq!(serialized, control);
    }

    #[concordium_test]
    fn test_serialize_single_key() {
        let prefix = Bytes(String::from("my_prefix").into_bytes());
        let key = String::from("my_key");

        let serialized = serial_single_key(prefix.as_ref(), &key);
        let control = to_bytes(&StorageKeys {
            prefix,
            keys: StorageKeySelection::Some(vec![Bytes(to_bytes(&key))]),
        });

        claim_eq!(serialized, control);
    }

    #[concordium_test]
    fn test_send_insert() {
        let mut host = TestHost::new(TestState(), TestStateBuilder::new());

        host.setup_mock_entrypoint(
            STORAGE_ADDR,
            OwnedEntrypointName::new_unchecked(String::from("insert")),
            parse_and_ok_mock::<StorageEntries, _>(()),
        );

        let result = host.storage_insert(
            &STORAGE_ADDR,
            Bytes::from([1, 2, 3]).as_ref(),
            &[4, 5, 6],
            &[7, 8, 9],
        );

        claim!(result.is_ok())
    }

    #[concordium_test]
    fn test_send_update() {
        let mut host = TestHost::new(TestState(), TestStateBuilder::new());

        host.setup_mock_entrypoint(
            STORAGE_ADDR,
            OwnedEntrypointName::new_unchecked(String::from("update")),
            parse_and_ok_mock::<StorageEntries, _>(()),
        );

        let result = host.storage_update(
            &STORAGE_ADDR,
            Bytes::from([1, 2, 3]).as_ref(),
            &[4, 5, 6],
            &[7, 8, 9],
        );

        claim!(result.is_ok())
    }

    #[concordium_test]
    fn test_send_remove() {
        let mut host = TestHost::new(TestState(), TestStateBuilder::new());

        host.setup_mock_entrypoint(
            STORAGE_ADDR,
            OwnedEntrypointName::new_unchecked(String::from("remove")),
            parse_and_ok_mock::<StorageKeys, _>(()),
        );

        let result =
            host.storage_remove(&STORAGE_ADDR, Bytes::from([1, 2, 3]).as_ref(), &[4, 5, 6]);

        claim!(result.is_ok());
    }

    #[concordium_test]
    fn test_send_get() {
        let mut host = TestHost::new(TestState(), TestStateBuilder::new());

        host.setup_mock_entrypoint(
            STORAGE_ADDR,
            OwnedEntrypointName::new_unchecked(String::from("get")),
            parse_and_ok_mock::<StorageKeys, _>(Some(StorageGetEntryResult {
                prefix: Bytes::from([1, 2, 3]),
                entries: vec![MaybeStorageEntry {
                    key: Bytes::from([4, 5, 6]),
                    value: Some(Bytes(to_bytes(&Bytes::from([7, 8, 9])))),
                }],
            })),
        );

        let result: Result<Option<Bytes>, _> =
            host.storage_get(&STORAGE_ADDR, Bytes::from([1, 2, 3]).as_ref(), &[4, 5, 6]);

        match result {
            Ok(res) => claim_eq!(res, Some(Bytes::from([7, 8, 9]))),
            Err(e) => fail!("Get should not fail: {:?}", e),
        }
    }
}
