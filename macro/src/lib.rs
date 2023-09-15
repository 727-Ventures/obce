// Copyright (c) 2012-2022 Supercolony
//
// Permission is hereby granted, free of charge, to any person obtaining
// a copy of this software and associated documentation files (the"Software"),
// to deal in the Software without restriction, including
// without limitation the rights to use, copy, modify, merge, publish,
// distribute, sublicense, and/or sell copies of the Software, and to
// permit persons to whom the Software is furnished to do so, subject to
// the following conditions:
//
// The above copyright notice and this permission notice shall be
// included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
// MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
// NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE
// LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION
// WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

#![cfg_attr(not(feature = "std"), no_std)]

use proc_macro::TokenStream;

use obce_codegen::{
    definition,
    error,
    extension,
    id,
    implementation,
    mock,
};

/// Chain extension definition for use with Substrate-based nodes and ink! smart contracts.
///
/// # Description
///
/// This macro generates code based on activated OBCE features.
///
/// When used with `ink` feature, [`#[obce::definition]`](macro@definition) generates
/// a glue code to correctly call your chain extension from ink! smart contracts.
///
/// The behaviour of [`#[obce::definition]`](macro@definition) with `substrate` feature enabled
/// is to leave everything as-is, without any additional modifications.
///
/// ```ignore
/// pub struct ChainExtension;
///
/// #[obce::definition]
/// pub trait ChainExtensionDefinition {
///     fn some_method(&self, argument: u32) -> u64;
/// }
/// ```
///
/// # Custom identifiers
///
/// You can use `#[obce::definition(id = ...)]` and `#[obce(id = ...)]` to override
/// the automatically generated chain extension identifier and chain extension method identifier
/// correspondingly.
///
/// `id` accepts literals of type [`&str`] and [`u16`].
#[proc_macro_attribute]
pub fn definition(attrs: TokenStream, trait_item: TokenStream) -> TokenStream {
    match definition::generate(attrs.into(), trait_item.into()) {
        Ok(traits) => traits.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Chain extension implementation for use with Substrate-based nodes.
///
/// # Description
///
/// This macro generates the necessary trait implementations for you to use
/// your chain extension with Substrate runtime.
///
/// This macro checks for the generics that you use in your impl block.
///
/// ```ignore
/// use obce::substrate::{
///     frame_system::Config as SysConfig,
///     pallet_contracts::Config as ContractConfig,
///     sp_runtime::traits::StaticLookup,
///     ChainExtensionEnvironment,
///     ExtensionContext
/// };
///
/// pub struct ChainExtension;
///
/// #[obce::definition]
/// pub trait ChainExtensionDefinition {
///     fn extension_method(&self);
/// }
///
/// #[obce::implementation]
/// impl<'a, E, T, Env> ChainExtensionDefinition for ExtensionContext<'a, E, T, Env, ChainExtension>
/// where
///     T: SysConfig + ContractConfig,
///     <<T as SysConfig>::Lookup as StaticLookup>::Source: From<<T as SysConfig>::AccountId>,
///     Env: ChainExtensionEnvironment<E, T>,
/// {
///     fn extension_method(&self) {
///         // Do awesome stuff!
///     }
/// }
/// ```
///
/// # Generics
///
/// `E` represents the external environment in which smart contracts are being executed.
/// When building chain extension without OBCE, it is usually bounded by `pallet_contracts::chain_extension::Ext`,
/// providing you access to methods that interacts with the execution environment. However,
/// to provide you with better testing capabilities OBCE does not bound the `E` generic itself,
/// resorting to bound the `Env` with it instead.
///
/// `T` represents your configuration type, which can be bounded by pallet-specific configuration traits
/// (such as `pallet_contracts::pallet::Config` and `frame_system::Config`).
///
/// `Env` generic is used to represent the OBCE-specific chain extension environment, which is more easily
/// testable, and can additionally be bounded by any trait you want to use. For example, you can add a trait that
/// represents your chain-specific pallet and use it inside of your chain extension.
///
/// # Weight charging
///
/// You can use `#[obce(weight(dispatch = ...))]` to automatically charge
/// weight based on a pallet call dispatch information.
///
/// `dispatch` accepts a full path to pallet's call (for example, `pallet_example::Pallet::<T>::my_call`).
///
/// OBCE will attempt to automatically obtain dispatch info based on the arguments passed
/// to your chain extension method.
///
/// If pallet's call arguments and your chain extension method
/// arguments are different, you can use `args` to override them:
/// `#[obce(weight(dispatch = "pallet_example::Pallet::<T>::my_call", args = "some_val,123"))]`.
///
/// You can also use `#[obce(weight(expr = ...))]` to charge weight without pallet calls.
/// In this case, you can simply provide any expression which returns `Weight`:
/// `#[obce(weight(expr = "Weight::from_parts(ref_time, proof_size)"))]`.
///
/// OBCE also provides you with a pre-charging feature, which charges weight before
/// any data parsing is done, making sure that weight is paid even if the call
/// is not successful:
///
/// ```ignore
/// use obce::substrate::{
///     sp_weights::Weight,
///     frame_system::Config as SysConfig,
///     pallet_contracts::Config as ContractConfig,
///     sp_runtime::traits::StaticLookup,
///     ChainExtensionEnvironment,
///     ExtensionContext
/// };
///
/// pub struct ChainExtension;
///
/// #[obce::definition]
/// pub trait ChainExtensionDefinition {
///     fn extension_method(&mut self, val: u64);
/// }
///
/// #[obce::implementation]
/// impl<'a, E, T, Env> ChainExtensionDefinition for ExtensionContext<'a, E, T, Env, ChainExtension>
/// where
///     T: SysConfig + ContractConfig,
///     <<T as SysConfig>::Lookup as StaticLookup>::Source: From<<T as SysConfig>::AccountId>,
///     Env: ChainExtensionEnvironment<E, T>,
/// {
///     #[obce(weight(expr = "Weight::from_parts(123, 0)", pre_charge))]
///     fn extension_method(&mut self, _val: u64) {
///         self.pre_charged().unwrap();
///     }
/// }
///
/// fn main() {}
/// ```
///
/// ## Usage example
///
/// ```ignore
/// use obce::substrate::{
///     frame_system::{Config as SysConfig, RawOrigin},
///     pallet_contracts::Config as ContractConfig,
///     sp_runtime::traits::StaticLookup,
///     ChainExtensionEnvironment,
///     ExtensionContext
/// };
///
/// pub struct ChainExtension;
///
/// #[obce::definition]
/// pub trait ChainExtensionDefinition {
///     fn extension_method(&mut self, val: u64);
/// }
///
/// #[obce::implementation]
/// impl<'a, E, T, Env> ChainExtensionDefinition for ExtensionContext<'a, E, T, Env, ChainExtension>
/// where
///     T: SysConfig + ContractConfig + pallet_example::Config,
///     <<T as SysConfig>::Lookup as StaticLookup>::Source: From<<T as SysConfig>::AccountId>,
///     Env: ChainExtensionEnvironment<E, T>,
/// {
///     #[obce(weight(dispatch = "pallet_example::Pallet::<T>::test_method", args = "123"))]
///     fn extension_method(&mut self, val: u64) {
///         // ...
///     }
/// }
/// ```
///
/// ## `Ext` trait bounds
///
/// You may notice that the example above doesn't have `E: Ext<T = T>` bound, which is required
/// when calling your chain extension via `pallet_contracts::chain_extension::ChainExtension`.
///
/// This is because OBCE automatically generates two separate trait implementations for your
/// chain extension struct - `obce::substrate::CallableChainExtension` and `pallet_contracts::chain_extension::ChainExtension`.
///
/// Only when generating the latter OBCE automatically adds `E: Ext<T = T>` bound, while still providing
/// you capabilities to manually add `E: Ext<T = T>` on the implementation trait bounds to allow `Ext` trait
/// usage inside implementation methods:
///
/// ```ignore
/// use obce::substrate::{
///     frame_system::{Config as SysConfig, RawOrigin},
///     pallet_contracts::{
///         chain_extension::Ext,
///         Config as ContractConfig,
///     },
///     sp_runtime::traits::StaticLookup,
///     ChainExtensionEnvironment,
///     ExtensionContext
/// };
///
/// pub struct ChainExtension;
///
/// #[obce::definition]
/// pub trait ChainExtensionDefinition {
///     fn extension_method(&mut self, val: u64);
/// }
///
/// #[obce::implementation]
/// impl<'a, E, T, Env> ChainExtensionDefinition for ExtensionContext<'a, E, T, Env, ChainExtension>
/// where
///     T: SysConfig + ContractConfig + pallet_example::Config,
///     <<T as SysConfig>::Lookup as StaticLookup>::Source: From<<T as SysConfig>::AccountId>,
///     Env: ChainExtensionEnvironment<E, T>,
///     E: Ext<T = T>,
/// {
///     fn extension_method(&mut self, val: u64) {
///         // Ext trait can be used here
///     }
/// }
/// ```
///
/// This is done to ease chain extension environment generalization during testing.
#[proc_macro_attribute]
pub fn implementation(attrs: TokenStream, impl_item: TokenStream) -> TokenStream {
    match implementation::generate(attrs.into(), impl_item.into()) {
        Ok(impls) => impls.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Chain extension error.
///
/// # Description
///
/// Using [`#[obce::error]`](macro@error) you can generate custom chain extension
/// errors.
///
/// Errors marked with [`#[obce::error]`](macro@error) have [`Debug`], [`Copy`], [`Clone`], [`PartialEq`], [`Eq`], `scale::Encode` and `scale::Decode`
/// automatically derived for them.
///
/// ```ignore
/// #[obce::error]
/// enum Error {
///     FirstError,
///     SecondError(u32)
/// }
/// ```
///
/// # Critical errors
///
/// [`#[obce::error]`](macro@error) can automatically generate `SupportCriticalError`
/// implementation for variant that you mark with `#[obce(critical)]`:
///
/// ```ignore
/// use obce::substrate::CriticalError;
///
/// #[obce::error]
/// enum Error {
///     FirstError,
///
///     #[obce(critical)]
///     Two(CriticalError)
/// }
/// ```
///
/// Only one enum variant can be marked as `#[obce(critical)]`.
///
/// # `RetVal`-convertible errors
///
/// You can mark error variants with `#[obce(ret_val = "...")]` to create an implementation of
/// [`TryFrom<YourError>`](::core::convert::TryFrom) for `pallet_contracts::chain_extension::RetVal`,
/// which will automatically convert suitable error variants to `RetVal` on implementation methods marked with `#[obce(ret_val)]`.
///
/// Error variant's `#[obce(ret_val = "...")]` accepts an expression that evaluates to [`u32`]:
///
/// ```ignore
/// #[obce::error]
/// enum Error {
///     #[obce(ret_val = "10_001")]
///     First,
///
///     Second
/// }
/// ```
#[proc_macro_attribute]
pub fn error(attrs: TokenStream, enum_item: TokenStream) -> TokenStream {
    match error::generate(attrs.into(), enum_item.into()) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Chain extension mocking utility.
///
/// # Description
///
/// You can use [`#[obce::mock]`](macro@mock) to automatically generate `register_chain_extensions`
/// function, which accepts a context and automatically registers mocked chain extension methods
/// for off-chain ink! smart contract testing.
///
/// Such a testing is useful to check smart contract's behaviour in the absence of
/// an available node.
///
/// ```ignore
/// // ink! smart contract definition is omitted.
///
/// #[obce::definition]
/// pub trait MyChainExtension {
///     fn test_method(&mut self, val: u32, another_val: u32) -> u32;
/// }
///
/// #[obce::mock]
/// impl MyChainExtension for () {
///     fn test_method(&mut self, val: u32, another_val: u32) -> u32 {
///         val + another_val
///     }
/// }
///
/// #[test]
/// fn call_contract() {
///     register_chain_extensions(());
///     let mut contract = SimpleContract::new();
///     assert_eq!(contract.call_test_method(100, 200), 300);
/// }
/// ```
///
/// When using [`#[obce::mock]`](macro@mock), you are not required to fill every single
/// method for testing. Glue code to register chain extension methods will only apply to
/// those methods, that you listed in a mock macro call:
///
/// ```ignore
/// #[obce::definition]
/// pub trait MyChainExtension {
///     fn first_method(&mut self, val: u32) -> u32;
///     fn second_method(&mut self) -> u64;
/// }
///
/// #[obce::mock]
/// impl MyChainExtension for () {
///     fn first_method(&mut self, val: u32) -> u32 {
///         // ...
///     }
///
///     // second_method is not required to be present here
/// }
/// ```
///
/// If an attempt is made to make a call to a missing method a panic with `UnregisteredChainExtension`
/// message will be issued.
///
/// # Context
///
/// The item that you implement your definition trait for becomes your testing context.
///
/// You will receive the same testing context when calling methods multiple times,
/// thus it can be used as your chain extension testing state:
///
/// ```ignore
/// #[obce::definition]
/// pub trait Trait {
///     fn method(&mut self) -> u32;
/// }
///
/// #[obce::ink_lang::extension]
/// struct TestExtension;
///
/// impl Trait for TestExtension {}
///
/// #[ink::contract]
/// mod simple_contract {
///     use crate::{
///         TestExtension,
///         Trait,
///     };
///
///     #[ink(storage)]
///     pub struct SimpleContract {}
///
///     impl SimpleContract {
///         #[ink(constructor)]
///         pub fn new() -> Self {
///             SimpleContract {}
///         }
///
///         #[ink(message)]
///         pub fn call_method(&mut self) -> u32 {
///             TestExtension.method()
///         }
///     }
/// }
///
/// mod state_test {
///     #[derive(Clone, Default)]
///     pub struct State {
///         call_count: u32,
///     }
///
///     #[obce::mock]
///     impl crate::Trait for State {
///         fn method(&mut self) -> u32 {
///             self.call_count += 1;
///             self.call_count
///         }
///     }
///
///     #[test]
///     fn call_contract() {
///         register_chain_extensions(State::default());
///         let mut contract = crate::simple_contract::SimpleContract::new();
///         assert_eq!(contract.call_method(), 1);
///         assert_eq!(contract.call_method(), 2);
///         assert_eq!(contract.call_method(), 3);
///     }
/// }
/// ```
///
/// # General guidelines
///
/// Since [`#[obce::mock]`](macro@mock) is designed for off-chain testing, you are
/// limited by off-chain testing facilities that [ink! library provides](https://use.ink/basics/contract-testing).
///
/// # Complete example
///
/// ```ignore
/// #[obce::definition(id = 123)]
/// pub trait ChainExtension {
///     fn method(&mut self, val: u32, another_val: u32) -> u32;
///
///     #[obce(id = 456)]
///     fn another_method(&mut self, val: u32) -> u32;
/// }
///
/// #[obce::ink_lang::extension]
/// struct MyChainExtension;
///
/// impl ChainExtension for MyChainExtension {}
///
/// #[ink::contract]
/// mod simple_contract {
///     use crate::{
///         ChainExtension,
///         MyChainExtension,
///     };
///
///     #[ink(storage)]
///     pub struct SimpleContract {}
///
///     impl SimpleContract {
///         #[ink(constructor)]
///         pub fn new() -> Self {
///             SimpleContract {}
///         }
///
///         #[ink(message)]
///         pub fn call_method(&mut self, val: u32, another_val: u32) -> u32 {
///             MyChainExtension.method(val, another_val)
///         }
///
///         #[ink(message)]
///         pub fn call_another_method(&mut self, val: u32) -> u32 {
///             MyChainExtension.another_method(val)
///         }
///     }
/// }
///
/// mod simple_test {
///     #[obce::mock]
///     impl crate::ChainExtension for () {
///         fn method(&mut self, val: u32, another_val: u32) -> u32 {
///             val + another_val
///         }
///     }
///
///     #[test]
///     fn call_contract() {
///         register_chain_extensions(());
///         let mut contract = crate::simple_contract::SimpleContract::new();
///         assert_eq!(contract.call_method(100, 200), 300);
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn mock(attrs: TokenStream, enum_item: TokenStream) -> TokenStream {
    match mock::generate(attrs.into(), enum_item.into()) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// ink! chain extension marker.
///
/// # Description
///
/// Using this macro, you can mark your ink! chain extension structs to
/// be instantiable using ink!'s environment.
///
/// # Example
///
/// ```ignore
/// #[obce::definition]
/// pub trait Trait {
///     fn method(&mut self, val: u32, another_val: u32) -> u32;
/// }
///
/// #[obce::ink_lang::extension]
/// struct TestExtension;
///
/// impl Trait for TestExtension {}
/// ```
///
/// # Usage with ink!
///
/// To integrate such an extension with ink!, you can use the following example:
///
/// ```ignore
/// use ink::env::{DefaultEnvironment, Environment};
///
/// #[derive(Debug, Clone, PartialEq, Eq)]
/// #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
/// pub enum CustomEnvironment {}
///
/// impl Environment for CustomEnvironment {
///     const MAX_EVENT_TOPICS: usize =
///         <DefaultEnvironment as Environment>::MAX_EVENT_TOPICS;
///
///     type AccountId = <DefaultEnvironment as Environment>::AccountId;
///     type Balance = <DefaultEnvironment as Environment>::Balance;
///     type Hash = <DefaultEnvironment as Environment>::Hash;
///     type BlockNumber = <DefaultEnvironment as Environment>::BlockNumber;
///     type Timestamp = <DefaultEnvironment as Environment>::Timestamp;
///
///     type ChainExtension = TestExtension;
/// }
/// ```
#[proc_macro_attribute]
pub fn ink_extension(attrs: TokenStream, struct_item: TokenStream) -> TokenStream {
    match extension::ink(attrs.into(), struct_item.into()) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Chain extension identifier lookup.
///
/// # Description
///
/// Using [`obce::id!`](macro@id) macro, you can lookup chain extension and chain extension method identifiers.
///
/// # Example
///
/// ```ignore
/// #[obce::definition(id = 123)]
/// pub trait ChainExtension {
///     #[obce(id = 456)]
///     fn method(&self);
/// }
///
/// assert_eq!(obce::id!(ChainExtension), 123);
/// assert_eq!(obce::id!(ChainExtension::method), 456);
/// ```
///
/// # Supported paths
///
/// To correctly distinguish between a chain extension itself and a chain extension method,
/// you have to provide a path with at most two segments (for example, `ChainExtension`, `SomeExtension::method`).
///
/// The macro will provide you with an error message in case if the path you provided is incorrect.
#[proc_macro]
pub fn id(path: TokenStream) -> TokenStream {
    match id::generate(path.into()) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}
