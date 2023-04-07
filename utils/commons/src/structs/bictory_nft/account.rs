use super::*;

/// The parameter type for the contract function `queryAccountInfo`.
#[derive(Serialize, SchemaType)]
pub struct AccountInfoQueryParams {
    /// The contract to trigger with the results of the query.
    pub result_contract: ContractAddress,
    /// The contract function to trigger with the results of the query.
    pub result_function: OwnedReceiveName,
}
