use commons_v1::{CustomContractError, Royalty, Token};
use concordium_cis1::{AdditionalData, Receiver, TokenIdVec, Transfer};
use concordium_std::*;

pub fn transfer<T>(
    host: &mut impl HasHost<T>,
    token: Token,
    from: Address,
    to: AccountAddress,
) -> ReceiveResult<()> {
    host.invoke_contract(
        &token.contract,
        &(
            1u16,
            Transfer {
                token_id: token.id,
                amount: 1,
                from,
                to: Receiver::Account(to),
                data: AdditionalData::empty(),
            },
        ),
        EntrypointName::new_unchecked("transfer"),
        Amount::zero(),
    )
    .map_err(handle_call_error)?;

    Ok(())
}

pub fn get_royalties<T>(
    host: &impl HasHost<T>,
    contract: &ContractAddress,
    token_id: &TokenIdVec,
) -> ReceiveResult<Vec<Royalty>> {
    let mut response = host
        .invoke_contract_read_only(
            contract,
            token_id,
            EntrypointName::new_unchecked("getRoyalties"),
            Amount::zero(),
        )
        .map_err(handle_call_error)?
        .ok_or(CustomContractError::Incompatible)?;

    <Vec<Royalty>>::deserial(&mut response).map_err(|_| CustomContractError::Incompatible.into())
}

fn handle_call_error<R>(error: CallContractError<R>) -> Reject {
    match error {
        CallContractError::MissingEntrypoint | CallContractError::MessageFailed => {
            CustomContractError::Incompatible.into()
        }
        CallContractError::LogicReject { .. } => CustomContractError::InvokeContractError.into(),
        e => e.into(),
    }
}

#[concordium_cfg_test]
mod tests {
    use commons_v1::{GetRoyaltiesParams, GetRoyaltiesResponse, Percentage};
    use concordium_cis1::{TokenIdVec, TransferParams};
    use concordium_std::test_infrastructure::*;

    use super::*;

    const NFT_CONTRACT: ContractAddress = ContractAddress {
        index: 1,
        subindex: 0,
    };

    const USER_1: AccountAddress = AccountAddress([1; 32]);

    #[concordium_test]
    fn test_transfer() {
        let state = ();
        let state_builder = TestStateBuilder::default();
        let mut host = TestHost::new(state, state_builder);

        host.setup_mock_entrypoint(
            NFT_CONTRACT,
            OwnedEntrypointName::new_unchecked("transfer".into()),
            MockFn::new_v1(|param, _, _, _| {
                TransferParams::<TokenIdVec>::deserial(&mut Cursor::new(param.as_ref()))
                    .map_err(|_| CallContractError::Trap)?;
                Ok((true, ()))
            }),
        );

        let response = transfer(
            &mut host,
            Token {
                contract: NFT_CONTRACT,
                id: TokenIdVec([1; 32].into()),
            },
            Address::Contract(NFT_CONTRACT),
            USER_1,
        );

        claim_eq!(response, Ok(()))
    }

    #[concordium_test]
    fn test_get_royalties() {
        let state = ();
        let state_builder = TestStateBuilder::default();
        let mut host = TestHost::new(state, state_builder);

        host.setup_mock_entrypoint(
            NFT_CONTRACT,
            OwnedEntrypointName::new_unchecked("getRoyalties".into()),
            MockFn::new_v1(|param, _, _, _| {
                GetRoyaltiesParams::deserial(&mut Cursor::new(param.as_ref()))
                    .map_err(|_| CallContractError::Trap)?;

                Ok((
                    false,
                    GetRoyaltiesResponse {
                        royalties: vec![Royalty {
                            beneficiary: USER_1,
                            percentage: Percentage::from_percent(2),
                        }],
                    },
                ))
            }),
        );

        let response = get_royalties(&mut host, &NFT_CONTRACT, &TokenIdVec([1; 32].into()));

        claim_eq!(
            response,
            Ok(vec![Royalty {
                beneficiary: USER_1,
                percentage: Percentage::from_percent(2),
            }],)
        );
    }
}
