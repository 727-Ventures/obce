#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod test_contract {
    use chain_extension::{
        ink::Extension,
        ChainExtension as _,
        Error,
    };
    use ink::env::DefaultEnvironment;

    #[ink(storage)]
    pub struct TestContract {}
    
    impl TestContract {
        #[ink(constructor)]
        pub fn new() -> Self {
            TestContract {}
        }

        #[ink(message)]
        pub fn successful_method(&mut self) -> Result<(), Error<DefaultEnvironment>> {
            Extension.successful_method()
        }

        #[ink(message)]
        pub fn erroneous_method(&mut self) -> Result<(), Error<DefaultEnvironment>> {
            Extension.erroneous_method()
        }

        #[ink(message)]
        pub fn critically_erroneous_method(&mut self) -> Result<(), Error<DefaultEnvironment>> {
            Extension.critically_erroneous_method()
        }

        #[ink(message)]
        pub fn multi_arg_method(&mut self, one: u64, two: u64) -> Result<u64, Error<DefaultEnvironment>> {
            Extension.multi_arg_method(one, two)
        }

        #[ink(message)]
        pub fn weight_linear_method(&mut self, complexity: u64) -> Result<(), Error<DefaultEnvironment>> {
            Extension.weight_linear_method(complexity)
        }
    }
}
