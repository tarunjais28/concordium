#![no_std]

#[macro_export]
macro_rules! proxy_contract {
    (
        contract: $contract:literal
        $($vis:vis $rust_func:ident => $contract_func:literal($contract_param:literal);)+
    ) => {
        use ::commons::ContractError;
        use ::concordium_std::*;

        type ContractResult<T> = Result<T, ContractError>;

        #[derive(Serialize, SchemaType)]
        pub struct InitParameter {
            target: ContractAddress,
            admins: HashSet<Address>,
            developers: HashSet<Address>,
        }

        #[derive(Serialize, SchemaType)]
        pub struct UserUpdateList {
            updates: Vec<(Address, RightsUpdate)>,
        }

        #[derive(Serialize, SchemaType)]
        enum RightsUpdate {
            Add,
            Remove,
        }

        #[derive(Serialize, SchemaType)]
        pub struct TargetContract {
            target: ContractAddress,
        }

        #[derive(Serialize, SchemaType)]
        pub struct State {
            target: ContractAddress,
            admins: HashSet<Address>,
            developers: HashSet<Address>,
        }

        impl State {
            fn forward<A: HasActions>(
                &self,
                function_name: &str,
                parameter_cursor: &mut impl HasParameter,
            ) -> Result<A, ContractError> {
                let target_function = ReceiveName::new_unchecked(function_name);

                let size = parameter_cursor.size() as usize;
                let mut parameter = Vec::with_capacity(size);
                // SAFETY:
                // * new length is equal to capacity, which is reserved on vector initialization
                // * all elements are initialized with `read_exact` function, otherwise error is returned and vector is dropped
                unsafe {
                    parameter.set_len(size);
                    parameter_cursor.read_exact(&mut parameter)?;
                }

                Ok(A::send_raw(&self.target, target_function, Amount::zero(), &parameter))
            }

            fn has_admin_rights(&self, addr: &Address) -> bool {
                self.admins.contains(addr)
            }

            fn has_dev_rights(&self, addr: &Address) -> bool {
                self.admins.contains(addr) || self.developers.contains(addr)
            }
        }

        /// Initialize proxy contract instance with originator as only admin
        #[init(contract = "BictoryNFT", parameter = "InitParameter")]
        pub fn contract_init(ctx: &impl HasInitContext) -> InitResult<State> {
            let mut params: InitParameter = ctx.parameter_cursor().get()?;
            if params.admins.is_empty() {
                params.admins.insert(Address::Account(ctx.init_origin()));
            }
            let state = State {
                target: params.target,
                admins: params.admins,
                developers: params.developers,
            };
            Ok(state)
        }

        #[receive(
            contract = "BictoryNFT",
            name = "proxyUpgradeTarget",
            parameter = "TargetContract"
        )]
        pub fn contract_upgrade_contract<A: HasActions>(
            ctx: &impl HasReceiveContext,
            state: &mut State,
        ) -> ContractResult<A> {
            if state.has_dev_rights(&ctx.sender()) {
                let params: TargetContract = ctx.parameter_cursor().get()?;
                state.target = params.target;
                Ok(A::accept())
            } else {
                Err(ContractError::Unauthorized)
            }
        }

        #[receive(
            contract = "BictoryNFT",
            name = "proxyUpdateAdmins",
            parameter = "UserUpdateList"
        )]
        pub fn contract_update_admins<A: HasActions>(
            ctx: &impl HasReceiveContext,
            state: &mut State,
        ) -> ContractResult<A> {
            if state.has_admin_rights(&ctx.sender()) {
                let params: UserUpdateList = ctx.parameter_cursor().get()?;
                for (user, update) in params.updates {
                    match update {
                        RightsUpdate::Add => state.admins.insert(user),
                        RightsUpdate::Remove => state.admins.remove(&user),
                    };
                }
                Ok(A::accept())
            } else {
                Err(ContractError::Unauthorized)
            }
        }

        #[receive(
            contract = "BictoryNFT",
            name = "proxyUpdateDevelopers",
            parameter = "UserUpdateList"
        )]
        pub fn contract_update_devs<A: HasActions>(
            ctx: &impl HasReceiveContext,
            state: &mut State,
        ) -> ContractResult<A> {
            if state.has_dev_rights(&ctx.sender()) {
                let params: UserUpdateList = ctx.parameter_cursor().get()?;
                for (user, update) in params.updates {
                    match update {
                        RightsUpdate::Add => state.developers.insert(user),
                        RightsUpdate::Remove => state.developers.remove(&user),
                    };
                }
                Ok(A::accept())
            } else {
                Err(ContractError::Unauthorized)
            }
        }

        $(
            #[receive(contract = $contract, name = $contract_func, parameter = $contract_param)]
            pub fn $rust_func<A: HasActions>(
                ctx: &impl HasReceiveContext,
                state: &mut State,
            ) -> ContractResult<A> {
                state.forward(
                    concat!($contract, ".", $contract_func),
                    &mut ctx.parameter_cursor(),
                )
            }
        )+
    };
    (contract: $contract:literal) => {
        compile_error!("Forwarded functions must be specified for a proxy contract");
    }
}
