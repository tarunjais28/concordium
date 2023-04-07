#![no_std]

use generic_proxy::proxy_contract;

#[allow(unused)]
use commons::{ContractBalanceOfQueryParams, ContractTokenId, MintParams, TransferParameter};
#[allow(unused)]
use concordium_cis2::{OperatorOfQueryParams, UpdateOperatorParams};

proxy_contract! {
    contract: "BictoryNFT"

    pub contract_mint => "mint"("MintParams");
    pub contract_transfer => "transfer"("TransferParameter");
    pub contract_update_operator => "updateOperator"("UpdateOperatorParams");
    pub contract_operator_of => "operatorOf"("OperatorOfQueryParams");
    pub contract_balance_of => "balanceOf"("ContractBalanceOfQueryParams");
    pub contract_burn => "burn"("ContractTokenId");
}
