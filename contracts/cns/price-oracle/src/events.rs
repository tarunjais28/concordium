use commons::SET_YEARLY_DOMAIN_PRICE_TAG;
use concordium_std::*;

use crate::external::PricingParams;

/// Tagged Custom event to be serialized for the event log.
#[derive(Debug)]
pub enum CnsPriceOracleEvent<'e> {
    /// Update the yearly domain pricing
    SetYearlyDomainPrice(&'e PricingParams),
}

impl<'e> Serial for CnsPriceOracleEvent<'e> {
    fn serial<W: Write>(&self, out: &mut W) -> Result<(), W::Err> {
        match self {
            CnsPriceOracleEvent::SetYearlyDomainPrice(pricing) => {
                out.write_u8(SET_YEARLY_DOMAIN_PRICE_TAG)?;
                pricing.serial(out)
            }
        }
    }
}
