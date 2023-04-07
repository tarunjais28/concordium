use concordium_std::concordium_cfg_test;

#[concordium_cfg_test]
pub use inner::*;

#[concordium_cfg_test]
mod inner {
    use concordium_std::test_infrastructure::MockFn;
    use concordium_std::*;

    pub fn parse_and_ok_mock<D: Deserial, S>(
        return_value: impl Clone + Serial + 'static,
    ) -> MockFn<S> {
        MockFn::new(move |parameter, _amount, _balance, _state| {
            D::deserial(&mut Cursor::new(parameter)).map_err(|_| CallContractError::Trap)?;
            Ok((false, Some(return_value.clone())))
        })
    }

    pub fn parse_and_check_mock<D: Deserial, S>(
        check: impl Fn(&D) -> bool + 'static,
        return_value: impl Clone + Serial + 'static,
    ) -> MockFn<S> {
        MockFn::new(move |parameter, _, _, _state| {
            let value =
                D::deserial(&mut Cursor::new(parameter)).map_err(|_| CallContractError::Trap)?;
            if !check(&value) {
                return Err(CallContractError::Trap);
            };
            Ok((false, Some(return_value.clone())))
        })
    }

    pub fn parse_and_map_mock<D: Deserial, T: Serial, S>(
        f: impl Fn(&D) -> Option<T> + 'static,
    ) -> MockFn<S> {
        MockFn::new(move |parameter, _, _, _state| {
            let value =
                D::deserial(&mut Cursor::new(parameter)).map_err(|_| CallContractError::Trap)?;
            f(&value)
                .map(|r| (false, Some(r)))
                .ok_or(CallContractError::Trap)
        })
    }
}
