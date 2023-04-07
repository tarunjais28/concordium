use concordium_std::*;

use crate::{
    CnsMintParams, ContractReadError, ContractTokenId, LendParams, TokenInfo, TokenParams,
    TokenSubscriptionStatus,
};

pub trait HostCnsNftExt<S>: HasHost<S> {
    fn cns_nft_mint(
        &mut self,
        contract: &ContractAddress,
        token_id: ContractTokenId,
        domain: String,
        owner: Address,
        duration: Duration,
    ) -> Result<(), CallContractError<Self::ReturnValueType>> {
        self.invoke_contract(
            contract,
            &CnsMintParams {
                token_id,
                domain,
                owner,
                duration,
            },
            EntrypointName::new_unchecked("mint"),
            Amount::zero(),
        )?;

        Ok(())
    }

    fn cns_nft_lend(
        &mut self,
        contract: &ContractAddress,
        token_id: ContractTokenId,
        extension: Duration,
    ) -> Result<(), CallContractError<Self::ReturnValueType>> {
        self.invoke_contract(
            contract,
            &LendParams {
                token_id,
                extension,
            },
            EntrypointName::new_unchecked("lend"),
            Amount::zero(),
        )?;

        Ok(())
    }

    fn cns_nft_burn(
        &mut self,
        contract: &ContractAddress,
        token_id: &ContractTokenId,
    ) -> Result<(), CallContractError<Self::ReturnValueType>> {
        self.invoke_contract(
            contract,
            token_id,
            EntrypointName::new_unchecked("burn"),
            Amount::zero(),
        )?;

        Ok(())
    }

    fn cns_nft_get_token_info(
        &self,
        contract: &ContractAddress,
        token_id: ContractTokenId,
    ) -> Result<Option<TokenInfo>, ContractReadError<Self::ReturnValueType>> {
        let mut result = self
            .invoke_contract_read_only(
                contract,
                &TokenParams { token_id },
                EntrypointName::new_unchecked("getTokenInfo"),
                Amount::zero(),
            )
            .map_err(ContractReadError::Call)?
            .ok_or(ContractReadError::Compatibility)?;

        let result =
            <Option<TokenInfo>>::deserial(&mut result).map_err(|_| ContractReadError::Parse)?;

        Ok(result)
    }

    fn cns_nft_get_token_expiry(
        &self,
        contract: &ContractAddress,
        token_id: ContractTokenId,
    ) -> Result<Option<TokenSubscriptionStatus>, ContractReadError<Self::ReturnValueType>> {
        let mut result = self
            .invoke_contract_read_only(
                contract,
                &TokenParams { token_id },
                EntrypointName::new_unchecked("getTokenExpiry"),
                Amount::zero(),
            )
            .map_err(ContractReadError::Call)?
            .ok_or(ContractReadError::Compatibility)?;

        let result = <Option<TokenSubscriptionStatus>>::deserial(&mut result)
            .map_err(|_| ContractReadError::Parse)?;

        Ok(result)
    }
}

impl<S, H: HasHost<S>> HostCnsNftExt<S> for H {}

#[concordium_cfg_test]
mod tests {
    // TODO: Take token_id by reference where possible after covering serialization with tests
}
