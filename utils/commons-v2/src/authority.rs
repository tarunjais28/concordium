use crate::CustomContractError;
use concordium_std::*;

#[derive(Debug, Serial, DeserialWithState, StateClone)]
#[concordium(state_parameter = "S")]
pub struct Authority<S: HasStateApi> {
    /// Trusted addresses that are allowed to maintain the contract and update admin and maintainer lists
    admins: StateSet<Address, S>,
    /// Semi-trusted addresses that are allowed to maintain the contract and update maintainer list
    maintainers: StateSet<Address, S>,
}

impl<S: HasStateApi> Authority<S> {
    pub fn new(state_builder: &mut StateBuilder<S>, admin: Address) -> Self {
        let mut admins = state_builder.new_set();
        admins.insert(admin);
        Self {
            admins,
            maintainers: state_builder.new_set(),
        }
    }

    pub fn has_admin_rights(&self, address: &Address) -> bool {
        self.admins.contains(address)
    }

    pub fn has_maintainer_rights(&self, address: &Address) -> bool {
        self.maintainers.contains(address) || self.has_admin_rights(address)
    }

    pub fn handle_update(
        &mut self,
        sender: Address,
        update: AuthorityUpdateParams,
    ) -> Result<(), Reject> {
        let address_list = match update.field {
            AuthorityField::Maintainer => {
                ensure!(
                    self.has_maintainer_rights(&sender),
                    CustomContractError::Unauthorized.into()
                );
                &mut self.maintainers
            }
            AuthorityField::Admin => {
                ensure!(
                    self.has_admin_rights(&sender),
                    CustomContractError::Unauthorized.into()
                );
                &mut self.admins
            }
        };

        match update.kind {
            AuthorityUpdateKind::Remove => {
                address_list.remove(&update.address);
            }
            AuthorityUpdateKind::Add => {
                address_list.insert(update.address);
            }
        }

        Ok(())
    }

    pub fn handle_view(&self, view: AuthorityViewParams) -> Vec<Address> {
        let address_list = match view.field {
            AuthorityField::Maintainer => &self.maintainers,
            AuthorityField::Admin => &self.admins,
        };

        let address_vec = address_list
            .iter()
            .skip(view.skip as usize)
            .take(view.show as usize)
            .map(|a| *a)
            .collect();

        address_vec
    }
}

#[derive(Debug, SchemaType, Serialize)]
pub enum AuthorityField {
    Maintainer,
    Admin,
}

#[derive(Debug, SchemaType, Serialize)]
pub enum AuthorityUpdateKind {
    Remove,
    Add,
}

#[derive(Debug, SchemaType, Serialize)]
pub struct AuthorityUpdateParams {
    pub field: AuthorityField,
    pub kind: AuthorityUpdateKind,
    pub address: Address,
}

#[derive(Debug, SchemaType, Serialize)]
pub struct AuthorityViewParams {
    pub field: AuthorityField,
    pub skip: u32,
    pub show: u32,
}

#[concordium_cfg_test]
mod tests {
    use super::*;
    use concordium_std::test_infrastructure::*;

    const ADMIN_ACCOUNT: AccountAddress = AccountAddress([1; 32]);
    const ADMIN_CONTRACT: ContractAddress = ContractAddress {
        index: 1,
        subindex: 1,
    };

    const MAINTAINER_ACCOUNT: AccountAddress = AccountAddress([2; 32]);
    const MAINTAINER_CONTRACT: ContractAddress = ContractAddress {
        index: 2,
        subindex: 2,
    };

    const USER_1: AccountAddress = AccountAddress([16; 32]);
    const USER_2: AccountAddress = AccountAddress([17; 32]);
    const CONTRACT_1: ContractAddress = ContractAddress {
        index: 16,
        subindex: 16,
    };
    const CONTRACT_2: ContractAddress = ContractAddress {
        index: 17,
        subindex: 17,
    };

    fn default_authority() -> Authority<TestStateApi> {
        let mut state_builder = TestStateBuilder::new();

        let mut authority = Authority::new(&mut state_builder, Address::Account(ADMIN_ACCOUNT));
        authority.admins.insert(Address::Contract(ADMIN_CONTRACT));

        authority
            .maintainers
            .insert(Address::Account(MAINTAINER_ACCOUNT));
        authority
            .maintainers
            .insert(Address::Contract(MAINTAINER_CONTRACT));

        authority
    }

    #[concordium_test]
    fn test_update_authority_add_new_admin() {
        let mut authority = default_authority();

        let result = authority.handle_update(
            Address::Account(ADMIN_ACCOUNT),
            AuthorityUpdateParams {
                field: AuthorityField::Admin,
                kind: AuthorityUpdateKind::Add,
                address: Address::Account(USER_1),
            },
        );
        claim_eq!(result, Ok(()));
        claim!(authority.has_admin_rights(&Address::Account(USER_1)));
        claim!(authority.has_maintainer_rights(&Address::Account(USER_1)));

        let result = authority.handle_update(
            Address::Contract(ADMIN_CONTRACT),
            AuthorityUpdateParams {
                field: AuthorityField::Admin,
                kind: AuthorityUpdateKind::Add,
                address: Address::Contract(CONTRACT_1),
            },
        );
        claim_eq!(result, Ok(()));
        claim!(authority.has_admin_rights(&Address::Contract(CONTRACT_1)));
        claim!(authority.has_maintainer_rights(&Address::Contract(CONTRACT_1)));

        let result = authority.handle_update(
            Address::Account(MAINTAINER_ACCOUNT),
            AuthorityUpdateParams {
                field: AuthorityField::Admin,
                kind: AuthorityUpdateKind::Add,
                address: Address::Account(USER_2),
            },
        );
        claim_eq!(result, Err(CustomContractError::Unauthorized.into()));
        claim!(!authority.has_admin_rights(&Address::Account(USER_2)));
        claim!(!authority.has_maintainer_rights(&Address::Account(USER_2)));

        let result = authority.handle_update(
            Address::Contract(MAINTAINER_CONTRACT),
            AuthorityUpdateParams {
                field: AuthorityField::Admin,
                kind: AuthorityUpdateKind::Add,
                address: Address::Contract(CONTRACT_2),
            },
        );
        claim_eq!(result, Err(CustomContractError::Unauthorized.into()));
        claim!(!authority.has_admin_rights(&Address::Contract(CONTRACT_2)));
        claim!(!authority.has_maintainer_rights(&Address::Contract(CONTRACT_2)));
    }

    #[concordium_test]
    fn test_update_authority_add_new_maintainer() {
        let mut authority = default_authority();

        let result = authority.handle_update(
            Address::Account(ADMIN_ACCOUNT),
            AuthorityUpdateParams {
                field: AuthorityField::Maintainer,
                kind: AuthorityUpdateKind::Add,
                address: Address::Account(USER_1),
            },
        );
        claim_eq!(result, Ok(()));
        claim!(!authority.has_admin_rights(&Address::Account(USER_1)));
        claim!(authority.has_maintainer_rights(&Address::Account(USER_1)));

        let result = authority.handle_update(
            Address::Contract(ADMIN_CONTRACT),
            AuthorityUpdateParams {
                field: AuthorityField::Maintainer,
                kind: AuthorityUpdateKind::Add,
                address: Address::Contract(CONTRACT_1),
            },
        );
        claim_eq!(result, Ok(()));
        claim!(!authority.has_admin_rights(&Address::Contract(CONTRACT_1)));
        claim!(authority.has_maintainer_rights(&Address::Contract(CONTRACT_1)));

        let result = authority.handle_update(
            Address::Account(MAINTAINER_ACCOUNT),
            AuthorityUpdateParams {
                field: AuthorityField::Maintainer,
                kind: AuthorityUpdateKind::Add,
                address: Address::Account(USER_2),
            },
        );
        claim_eq!(result, Ok(()));
        claim!(!authority.has_admin_rights(&Address::Account(USER_2)));
        claim!(authority.has_maintainer_rights(&Address::Account(USER_2)));

        let result = authority.handle_update(
            Address::Contract(MAINTAINER_CONTRACT),
            AuthorityUpdateParams {
                field: AuthorityField::Maintainer,
                kind: AuthorityUpdateKind::Add,
                address: Address::Contract(CONTRACT_2),
            },
        );
        claim_eq!(result, Ok(()));
        claim!(!authority.has_admin_rights(&Address::Contract(CONTRACT_2)));
        claim!(authority.has_maintainer_rights(&Address::Contract(CONTRACT_2)));
    }

    #[concordium_test]
    fn test_update_authority_add_existing_admin() {
        let mut authority = default_authority();

        let result = authority.handle_update(
            Address::Account(ADMIN_ACCOUNT),
            AuthorityUpdateParams {
                field: AuthorityField::Admin,
                kind: AuthorityUpdateKind::Add,
                address: Address::Contract(ADMIN_CONTRACT),
            },
        );
        // No change or error expected
        claim_eq!(result, Ok(()));
        claim!(authority.has_admin_rights(&Address::Contract(ADMIN_CONTRACT)));
        claim!(authority.has_maintainer_rights(&Address::Contract(ADMIN_CONTRACT)));

        let result = authority.handle_update(
            Address::Contract(ADMIN_CONTRACT),
            AuthorityUpdateParams {
                field: AuthorityField::Admin,
                kind: AuthorityUpdateKind::Add,
                address: Address::Account(ADMIN_ACCOUNT),
            },
        );
        // No change or error expected
        claim_eq!(result, Ok(()));
        claim!(authority.has_admin_rights(&Address::Account(ADMIN_ACCOUNT)));
        claim!(authority.has_maintainer_rights(&Address::Account(ADMIN_ACCOUNT)));

        let result = authority.handle_update(
            Address::Account(MAINTAINER_ACCOUNT),
            AuthorityUpdateParams {
                field: AuthorityField::Admin,
                kind: AuthorityUpdateKind::Add,
                address: Address::Account(ADMIN_ACCOUNT),
            },
        );
        claim_eq!(result, Err(CustomContractError::Unauthorized.into()));
        claim!(authority.has_admin_rights(&Address::Account(ADMIN_ACCOUNT)));
        claim!(authority.has_maintainer_rights(&Address::Account(ADMIN_ACCOUNT)));

        let result = authority.handle_update(
            Address::Contract(MAINTAINER_CONTRACT),
            AuthorityUpdateParams {
                field: AuthorityField::Admin,
                kind: AuthorityUpdateKind::Add,
                address: Address::Contract(ADMIN_CONTRACT),
            },
        );
        claim_eq!(result, Err(CustomContractError::Unauthorized.into()));
        claim!(authority.has_admin_rights(&Address::Contract(ADMIN_CONTRACT)));
        claim!(authority.has_maintainer_rights(&Address::Contract(ADMIN_CONTRACT)));
    }

    #[concordium_test]
    fn test_update_authority_add_existing_maintainer() {
        let mut authority = default_authority();

        let result = authority.handle_update(
            Address::Account(ADMIN_ACCOUNT),
            AuthorityUpdateParams {
                field: AuthorityField::Maintainer,
                kind: AuthorityUpdateKind::Add,
                address: Address::Account(MAINTAINER_ACCOUNT),
            },
        );
        // No change or error expected
        claim_eq!(result, Ok(()));
        claim!(!authority.has_admin_rights(&Address::Account(MAINTAINER_ACCOUNT)));
        claim!(authority.has_maintainer_rights(&Address::Account(MAINTAINER_ACCOUNT)));

        let result = authority.handle_update(
            Address::Contract(ADMIN_CONTRACT),
            AuthorityUpdateParams {
                field: AuthorityField::Maintainer,
                kind: AuthorityUpdateKind::Add,
                address: Address::Contract(MAINTAINER_CONTRACT),
            },
        );
        // No change or error expected
        claim_eq!(result, Ok(()));
        claim!(!authority.has_admin_rights(&Address::Contract(MAINTAINER_CONTRACT)));
        claim!(authority.has_maintainer_rights(&Address::Contract(MAINTAINER_CONTRACT)));

        let result = authority.handle_update(
            Address::Account(MAINTAINER_ACCOUNT),
            AuthorityUpdateParams {
                field: AuthorityField::Maintainer,
                kind: AuthorityUpdateKind::Add,
                address: Address::Contract(MAINTAINER_CONTRACT),
            },
        );
        // No change or error expected
        claim_eq!(result, Ok(()));
        claim!(!authority.has_admin_rights(&Address::Contract(MAINTAINER_CONTRACT)));
        claim!(authority.has_maintainer_rights(&Address::Contract(MAINTAINER_CONTRACT)));

        let result = authority.handle_update(
            Address::Contract(MAINTAINER_CONTRACT),
            AuthorityUpdateParams {
                field: AuthorityField::Maintainer,
                kind: AuthorityUpdateKind::Add,
                address: Address::Account(MAINTAINER_ACCOUNT),
            },
        );
        // No change or error expected
        claim_eq!(result, Ok(()));
        claim!(!authority.has_admin_rights(&Address::Account(MAINTAINER_ACCOUNT)));
        claim!(authority.has_maintainer_rights(&Address::Account(MAINTAINER_ACCOUNT)));
    }

    #[concordium_test]
    fn test_update_authority_remove_existing_admin() {
        let mut authority = default_authority();

        let result = authority.handle_update(
            Address::Account(ADMIN_ACCOUNT),
            AuthorityUpdateParams {
                field: AuthorityField::Admin,
                kind: AuthorityUpdateKind::Remove,
                address: Address::Contract(ADMIN_CONTRACT),
            },
        );
        claim_eq!(result, Ok(()));
        claim!(!authority.has_admin_rights(&Address::Contract(ADMIN_CONTRACT)));
        claim!(!authority.has_maintainer_rights(&Address::Contract(ADMIN_CONTRACT)));

        let result = authority.handle_update(
            Address::Account(MAINTAINER_ACCOUNT),
            AuthorityUpdateParams {
                field: AuthorityField::Admin,
                kind: AuthorityUpdateKind::Remove,
                address: Address::Account(ADMIN_ACCOUNT),
            },
        );
        claim_eq!(result, Err(CustomContractError::Unauthorized.into()));
        claim!(authority.has_admin_rights(&Address::Account(ADMIN_ACCOUNT)));
        claim!(authority.has_maintainer_rights(&Address::Account(ADMIN_ACCOUNT)));
    }

    #[concordium_test]
    fn test_update_authority_remove_existing_maintainer() {
        let mut authority = default_authority();

        let result = authority.handle_update(
            Address::Account(ADMIN_ACCOUNT),
            AuthorityUpdateParams {
                field: AuthorityField::Maintainer,
                kind: AuthorityUpdateKind::Remove,
                address: Address::Account(MAINTAINER_ACCOUNT),
            },
        );
        claim_eq!(result, Ok(()));
        claim!(!authority.has_admin_rights(&Address::Account(MAINTAINER_ACCOUNT)));
        claim!(!authority.has_maintainer_rights(&Address::Account(MAINTAINER_ACCOUNT)));

        let result = authority.handle_update(
            Address::Contract(MAINTAINER_CONTRACT),
            AuthorityUpdateParams {
                field: AuthorityField::Maintainer,
                kind: AuthorityUpdateKind::Remove,
                address: Address::Contract(MAINTAINER_CONTRACT),
            },
        );
        claim_eq!(result, Ok(()));
        claim!(!authority.has_admin_rights(&Address::Contract(MAINTAINER_CONTRACT)));
        claim!(!authority.has_maintainer_rights(&Address::Contract(MAINTAINER_CONTRACT)));
    }

    #[concordium_test]
    fn test_update_authority_remove_missing_admin() {
        let mut authority = default_authority();

        let result = authority.handle_update(
            Address::Account(ADMIN_ACCOUNT),
            AuthorityUpdateParams {
                field: AuthorityField::Admin,
                kind: AuthorityUpdateKind::Remove,
                address: Address::Contract(CONTRACT_1),
            },
        );
        // No change or error expected
        claim_eq!(result, Ok(()));
        claim!(!authority.has_admin_rights(&Address::Contract(CONTRACT_1)));
        claim!(!authority.has_maintainer_rights(&Address::Contract(CONTRACT_1)));

        let result = authority.handle_update(
            Address::Account(MAINTAINER_ACCOUNT),
            AuthorityUpdateParams {
                field: AuthorityField::Admin,
                kind: AuthorityUpdateKind::Remove,
                address: Address::Account(USER_1),
            },
        );
        claim_eq!(result, Err(CustomContractError::Unauthorized.into()));
        claim!(!authority.has_admin_rights(&Address::Account(USER_1)));
        claim!(!authority.has_maintainer_rights(&Address::Account(USER_1)));
    }

    #[concordium_test]
    fn test_update_authority_remove_missing_maintainer() {
        let mut authority = default_authority();

        let result = authority.handle_update(
            Address::Account(ADMIN_ACCOUNT),
            AuthorityUpdateParams {
                field: AuthorityField::Maintainer,
                kind: AuthorityUpdateKind::Remove,
                address: Address::Account(USER_1),
            },
        );
        // No change or error expected
        claim_eq!(result, Ok(()));
        claim!(!authority.has_admin_rights(&Address::Account(USER_1)));
        claim!(!authority.has_maintainer_rights(&Address::Account(USER_1)));

        let result = authority.handle_update(
            Address::Contract(MAINTAINER_CONTRACT),
            AuthorityUpdateParams {
                field: AuthorityField::Maintainer,
                kind: AuthorityUpdateKind::Remove,
                address: Address::Contract(CONTRACT_1),
            },
        );
        // No change or error expected
        claim_eq!(result, Ok(()));
        claim!(!authority.has_admin_rights(&Address::Contract(CONTRACT_1)));
        claim!(!authority.has_maintainer_rights(&Address::Contract(CONTRACT_1)));
    }

    #[concordium_test]
    fn test_view_authority_admins() {
        let mut authority = default_authority();

        let mut admin_set = (16u8..=255u8)
            .map(|n| {
                if n % 2 == 0 {
                    Address::Contract(ContractAddress {
                        index: n as u64,
                        subindex: 0,
                    })
                } else {
                    Address::Account(AccountAddress([n; 32]))
                }
            })
            .chain([
                Address::Account(ADMIN_ACCOUNT),
                Address::Contract(ADMIN_CONTRACT),
            ])
            .collect::<HashSet<_>>();

        for admin in admin_set.iter() {
            authority.admins.insert(*admin);
        }

        let mut num_seen = 0;
        let increment = 30;
        loop {
            let returned_addresses = authority.handle_view(AuthorityViewParams {
                field: AuthorityField::Admin,
                skip: num_seen,
                show: increment,
            });

            for addr in returned_addresses.iter() {
                // Check if the entry was present and remove it from the set. After the loop check that all addresses
                // were shown by confirming that `admin_set` is empty
                claim!(admin_set.remove(addr));
            }

            // If returned address count is less than `show`, no more addresses will be returned in future iterations
            if returned_addresses.len() != increment as usize {
                break;
            }
            num_seen += increment;
        }

        // All addresses must have been removed in the loop
        claim!(admin_set.is_empty());
    }
}
