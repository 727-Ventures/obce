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

mod environment;
mod is_critical_error;

pub use environment::ChainExtensionEnvironment;
pub use frame_support;
pub use frame_system;
pub use is_critical_error::{
    ToCriticalErr,
    ToCriticalErrFallback,
};
pub use pallet_contracts;
pub use sp_core;
pub use sp_runtime;
pub use sp_std;
pub use sp_weights;

use core::marker::PhantomData;

use pallet_contracts::chain_extension::RetVal;
use sp_runtime::DispatchError;

/// Callable chain extension with generalized environment information.
///
/// Unlike [`ChainExtension`](pallet_contracts::chain_extension::ChainExtension), [`CallableChainExtension`]
/// provides capabilities to call chain extensions with generalized [`ChainExtensionEnvironment`],
/// improving chain extension testing capabilities on the Substrate side.
///
/// This trait is automatically implemented on your Substrate chain extension struct
/// with [`#[obce::implementation]`](macro@crate::implementation) expansion.
///
/// # Generics
///
/// This trait mimics the generics that are used with [`#[obce::implementation]`](macro@crate::implementation) macro.
pub trait CallableChainExtension<E, T, Env> {
    /// Call chain extension with the provided [`ChainExtensionEnvironment`] implementation.
    fn call(&mut self, env: Env) -> Result<RetVal, CriticalError>;
}

/// Chain extension context that you can use with your implementations.
///
/// # Generics
///
/// This struct mimics the generics that are used with [`#[obce::implementation]`](macro@crate::implementation) macro.
pub struct ExtensionContext<'a, E, T, Env, Extension>
where
    Env: ChainExtensionEnvironment<E, T>,
{
    /// Chain extension environment.
    pub env: Env,

    /// Custom chain extension storage.
    pub storage: &'a mut Extension,

    pre_charged: Option<Env::ChargedAmount>,

    _ghost: PhantomData<(E, T)>,
}

impl<'a, E, T, Env, Extension> ExtensionContext<'a, E, T, Env, Extension>
where
    Env: ChainExtensionEnvironment<E, T>,
{
    pub fn new(storage: &'a mut Extension, env: Env, pre_charged: Option<Env::ChargedAmount>) -> Self {
        ExtensionContext {
            env,
            storage,
            pre_charged,
            _ghost: PhantomData,
        }
    }

    /// Get the pre-charged amount of weight.
    ///
    /// This method returns [`None`] if there was no pre-charge made,
    /// or if you previously called this method, since there is no copy
    /// of a charged amount involved.
    pub fn pre_charged(&mut self) -> Option<Env::ChargedAmount> {
        self.pre_charged.take()
    }
}

pub type CriticalError = DispatchError;

/// The trait allows filtering error on critical and non-critical errors.
///
/// Critical errors terminate the execution of the chain extension, while
/// non-critical errors are propagated to the caller contract via buffer.
pub trait SupportCriticalError: Sized {
    /// Try to convert an error to a critical one.
    ///
    /// Returns [`Ok`] if the conversion was successful (and the current
    /// error should be qualified as critical), and [`Err`] otherwise.
    fn try_to_critical(self) -> Result<CriticalError, Self>;
}
