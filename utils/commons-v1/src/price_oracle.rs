use concordium_std::*;

use super::{DomainKind, DomainPrice, GetDomainPriceParams, GetDomainPriceResult};
use crate::ContractReadError;

pub trait HostCnsOracleExt<S>: HasHost<S> {
    fn cns_get_yearly_domain_price(
        &self,
        contract: &ContractAddress,
        domain_kind: DomainKind,
        length: u16,
    ) -> Result<DomainPrice, ContractReadError<Self::ReturnValueType>> {
        let mut result = self
            .invoke_contract_read_only(
                contract,
                &GetDomainPriceParams {
                    domain_kind,
                    length,
                },
                EntrypointName::new_unchecked("getYearlyDomainPrice"),
                Amount::zero(),
            )
            .map_err(ContractReadError::Call)?
            .ok_or(ContractReadError::Compatibility)?;

        let result =
            GetDomainPriceResult::deserial(&mut result).map_err(|_| ContractReadError::Parse)?;

        Ok(result.result)
    }
}

impl<S, H: HasHost<S>> HostCnsOracleExt<S> for H {}
