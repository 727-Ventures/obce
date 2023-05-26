#![cfg_attr(not(feature = "std"), no_std, no_main)]

use ink::env::{DefaultEnvironment, Environment};

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum CustomEnvironment {}

impl Environment for CustomEnvironment {
    const MAX_EVENT_TOPICS: usize =
        <DefaultEnvironment as Environment>::MAX_EVENT_TOPICS;

    type AccountId = <DefaultEnvironment as Environment>::AccountId;
    type Balance = <DefaultEnvironment as Environment>::Balance;
    type Hash = <DefaultEnvironment as Environment>::Hash;
    type BlockNumber = <DefaultEnvironment as Environment>::BlockNumber;
    type Timestamp = <DefaultEnvironment as Environment>::Timestamp;

    type ChainExtension = rand_extension::ink::Extension;
}

#[ink::contract(env = crate::CustomEnvironment)]
mod test_contract {
    use rand_extension::{RandExtension, RandomReadErr};

    #[ink(storage)]
    pub struct TestContract {
        value: [u8; 32]
    }

    #[ink(event)]
    pub struct RandomUpdated {
        #[ink(topic)]
        new: [u8; 32],
    }

    impl TestContract {
        #[ink(constructor)]
        pub fn new(value: [u8; 32]) -> Self {
            Self { value }
        }

        #[ink(constructor)]
        pub fn new_default() -> Self {
            Self::new(Default::default())
        }

        #[ink(message)]
        pub fn update(&mut self, val: [u8; 32]) -> Result<(), RandomReadErr> {
            let new_random = self.env().extension().fetch_random(val)?;
            self.value = new_random;
            self.env().emit_event(RandomUpdated { new: new_random });
            Ok(())
        }

        #[ink(message)]
        pub fn get(&self) -> [u8; 32] {
            self.value
        }
    }

    #[cfg(test)]
    mod tests {
        use rand_extension::{RandExtension, RandomReadErr};

        use super::TestContract;

        pub struct Context;

        #[obce::mock]
        impl RandExtension for Context {
            fn fetch_random(&self, subject: [u8; 32]) -> Result<[u8; 32], RandomReadErr> {
                // Just pass the subject back for test purposes
                Ok(subject)
            }
        }

        #[ink::test]
        fn default_works() {
            let contract = TestContract::new_default();
            assert_eq!(contract.get(), [0; 32]);
        }

        #[ink::test]
        fn update_works() {
            register_chain_extensions(Context);
            let mut contract = TestContract::new_default();
            contract.update([1; 32]).unwrap();
            assert_eq!(contract.get(), [1; 32]);
        }
    }
}
