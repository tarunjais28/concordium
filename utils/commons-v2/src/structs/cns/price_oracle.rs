use concordium_std::*;

#[derive(Debug, Serialize, SchemaType)]
pub enum DomainKind {
    Domain,
    Subdomain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, SchemaType)]
pub enum DomainPrice {
    Limited,
    Amount(Amount),
}

#[derive(Serialize, SchemaType)]
pub struct GetDomainPriceParams {
    pub domain_kind: DomainKind,
    pub length: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, SchemaType)]
pub struct GetDomainPriceResult {
    pub result: DomainPrice,
}
