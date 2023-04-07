use crate::events::CnsPriceOracleEvent;
use crate::external::*;
use crate::state::{DomainPricing, State};
use commons::{
    AuthorityUpdateParams, AuthorityViewParams, CustomContractError, DomainKind,
    GetDomainPriceParams, GetDomainPriceResult,
};
use concordium_std::*;

/// Initialize contract instance with initial price.
#[init(contract = "BictoryCnsPriceOracle", parameter = "PricingParams")]
fn init<S: HasStateApi>(
    ctx: &impl HasInitContext,
    state_builder: &mut StateBuilder<S>,
) -> InitResult<State<S>> {
    let params = PricingParams::deserial(&mut ctx.parameter_cursor())?;

    // Construct the initial contract state.
    let state = State::new(state_builder, params, ctx.init_origin());

    Ok(state)
}

/// Function to set yearly domain price.
///
/// It rejects if:
/// - Fails to parse parameter;
/// - Fails to log `SetYearlyDomainPrice` event;
/// - Sender does not have maintainer rights.
#[receive(
    mutable,
    contract = "BictoryCnsPriceOracle",
    name = "setYearlyDomainPrice",
    parameter = "PricingParams",
    enable_logger
)]
fn set_yearly_domain_price<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
    logger: &mut impl HasLogger,
) -> ReceiveResult<()> {
    let params = PricingParams::deserial(&mut ctx.parameter_cursor())?;
    let state = host.state_mut();

    ensure!(
        state.authority.has_maintainer_rights(&ctx.sender()),
        CustomContractError::Unauthorized.into()
    );

    // Event for updated yearly domain price.
    logger.log(&CnsPriceOracleEvent::SetYearlyDomainPrice(&params))?;

    state.domain_prices = params.domain_pricing;
    state.subdomain_prices = params.subdomain_pricing;

    Ok(())
}

/// Function to get yearly domain price.
///
/// It rejects if:
/// - Fails to parse parameter.
#[receive(
    mutable,
    contract = "BictoryCnsPriceOracle",
    name = "getYearlyDomainPrice",
    parameter = "GetDomainPriceParams",
    return_value = "GetDomainPriceResult"
)]
fn get_yearly_domain_price<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ReceiveResult<GetDomainPriceResult> {
    let params = GetDomainPriceParams::deserial(&mut ctx.parameter_cursor())?;
    let state = host.state();

    let pricing = match params.domain_kind {
        DomainKind::Domain => &state.domain_prices,
        DomainKind::Subdomain => &state.subdomain_prices,
    };

    let result = match pricing {
        DomainPricing::Fixed(amount) => *amount,
        DomainPricing::Scaling(pricing) => {
            if params.length <= pricing.short_max_length {
                pricing.short
            } else if params.length > pricing.short_max_length + pricing.mid.len() as u16 {
                pricing.long
            } else {
                pricing.mid[(params.length - pricing.short_max_length - 1) as usize]
            }
        }
    };

    Ok(GetDomainPriceResult { result })
}

#[receive(
    mutable,
    contract = "BictoryCnsPriceOracle",
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
    contract = "BictoryCnsPriceOracle",
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

#[concordium_cfg_test]
mod tests {
    use commons::DomainPrice;
    use concordium_std::*;
    use test_infrastructure::*;

    use crate::state::ScalingPricing;

    use super::*;

    const ADMIN: AccountAddress = AccountAddress([1; 32]);
    const MAINTAINER: AccountAddress = AccountAddress([2; 32]);

    fn new_host_with(params: PricingParams) -> TestHost<State<TestStateApi>> {
        let mut ctx = TestInitContext::empty();
        let bytes = to_bytes(&params);
        ctx.set_init_origin(ADMIN).set_parameter(&bytes);
        let mut state_builder = TestStateBuilder::new();

        let state = init(&ctx, &mut state_builder)
            .expect_report("Failed during init_BictoryCnsPriceOracle");

        let mut host = TestHost::new(state, state_builder);

        let mut ctx = TestReceiveContext::empty();
        let params = AuthorityUpdateParams {
            field: commons::AuthorityField::Maintainer,
            kind: commons::AuthorityUpdateKind::Add,
            address: Address::Account(MAINTAINER),
        };
        let bytes = to_bytes(&params);
        ctx.set_sender(Address::Account(ADMIN))
            .set_parameter(&bytes);
        let result = update_authority(&ctx, &mut host);
        claim_eq!(result, Ok(()));

        host
    }

    #[concordium_test]
    fn test_fixed_pricing() {
        let mut host = new_host_with(PricingParams {
            domain_pricing: DomainPricing::Fixed(DomainPrice::Amount(Amount::from_ccd(10))),
            subdomain_pricing: DomainPricing::Fixed(DomainPrice::Amount(Amount::from_ccd(5))),
        });

        let mut ctx = TestReceiveContext::default();
        let params = GetDomainPriceParams {
            domain_kind: DomainKind::Domain,
            length: 5,
        };
        let bytes = to_bytes(&params);
        ctx.set_parameter(&bytes);

        let result = get_yearly_domain_price(&ctx, &mut host)
            .expect_report("Failed to call getYearlyDomainPrice");

        claim_eq!(
            result,
            GetDomainPriceResult {
                result: DomainPrice::Amount(Amount::from_ccd(10))
            }
        );

        let mut ctx = TestReceiveContext::default();
        let params = GetDomainPriceParams {
            domain_kind: DomainKind::Subdomain,
            length: 1,
        };
        let bytes = to_bytes(&params);
        ctx.set_parameter(&bytes);

        let result = get_yearly_domain_price(&ctx, &mut host)
            .expect_report("Failed to call getYearlyDomainPrice");

        claim_eq!(
            result,
            GetDomainPriceResult {
                result: DomainPrice::Amount(Amount::from_ccd(5))
            }
        );
    }

    #[concordium_test]
    fn test_scaling_pricing() {
        let mut host = new_host_with(PricingParams {
            domain_pricing: DomainPricing::Scaling(ScalingPricing {
                short_max_length: 3,
                short: DomainPrice::Limited,
                mid: vec![
                    DomainPrice::Amount(Amount::from_ccd(25)),
                    DomainPrice::Amount(Amount::from_ccd(20)),
                    DomainPrice::Amount(Amount::from_ccd(15)),
                    DomainPrice::Amount(Amount::from_ccd(10)),
                ],
                long: DomainPrice::Amount(Amount::from_ccd(5)),
            }),
            subdomain_pricing: DomainPricing::Fixed(DomainPrice::Amount(Amount::from_ccd(2))),
        });

        let mut ctx = TestReceiveContext::default();
        let params = GetDomainPriceParams {
            domain_kind: DomainKind::Domain,
            length: 3,
        };
        let bytes = to_bytes(&params);
        ctx.set_parameter(&bytes);

        let result = get_yearly_domain_price(&ctx, &mut host)
            .expect_report("Failed to call getYearlyDomainPrice");

        claim_eq!(
            result,
            GetDomainPriceResult {
                result: DomainPrice::Limited
            }
        );

        let mut ctx = TestReceiveContext::default();
        let params = GetDomainPriceParams {
            domain_kind: DomainKind::Domain,
            length: 4,
        };
        let bytes = to_bytes(&params);
        ctx.set_parameter(&bytes);

        let result = get_yearly_domain_price(&ctx, &mut host)
            .expect_report("Failed to call getYearlyDomainPrice");

        claim_eq!(
            result,
            GetDomainPriceResult {
                result: DomainPrice::Amount(Amount::from_ccd(25))
            }
        );

        let mut ctx = TestReceiveContext::default();
        let params = GetDomainPriceParams {
            domain_kind: DomainKind::Domain,
            length: 7,
        };
        let bytes = to_bytes(&params);
        ctx.set_parameter(&bytes);

        let result = get_yearly_domain_price(&ctx, &mut host)
            .expect_report("Failed to call getYearlyDomainPrice");

        claim_eq!(
            result,
            GetDomainPriceResult {
                result: DomainPrice::Amount(Amount::from_ccd(10))
            }
        );

        let mut ctx = TestReceiveContext::default();
        let params = GetDomainPriceParams {
            domain_kind: DomainKind::Domain,
            length: 8,
        };
        let bytes = to_bytes(&params);
        ctx.set_parameter(&bytes);

        let result = get_yearly_domain_price(&ctx, &mut host)
            .expect_report("Failed to call getYearlyDomainPrice");

        claim_eq!(
            result,
            GetDomainPriceResult {
                result: DomainPrice::Amount(Amount::from_ccd(5))
            }
        );

        let mut ctx = TestReceiveContext::default();
        let params = GetDomainPriceParams {
            domain_kind: DomainKind::Subdomain,
            length: 1,
        };
        let bytes = to_bytes(&params);
        ctx.set_parameter(&bytes);

        let result = get_yearly_domain_price(&ctx, &mut host)
            .expect_report("Failed to call getYearlyDomainPrice");

        claim_eq!(
            result,
            GetDomainPriceResult {
                result: DomainPrice::Amount(Amount::from_ccd(2))
            }
        );
    }
}
