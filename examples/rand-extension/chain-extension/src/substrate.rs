use obce::substrate::{
    frame_support::traits::Randomness,
    frame_system::Config as SysConfig,
    pallet_contracts::Config as ContractConfig,
    sp_core::H256,
    sp_runtime::traits::StaticLookup,
    ChainExtensionEnvironment,
    ExtensionContext,
};
use pallet_insecure_randomness_collective_flip::Config as RandomnessConfig;

use crate::{RandExtension, RandomReadErr};

#[derive(Default)]
pub struct Extension {}

#[obce::implementation]
impl<'a, E, T, Env> RandExtension for ExtensionContext<'a, E, T, Env, Extension>
where
    T: SysConfig<Hash = H256> + ContractConfig + RandomnessConfig,
    <<T as SysConfig>::Lookup as StaticLookup>::Source: From<<T as SysConfig>::AccountId>,
    Env: ChainExtensionEnvironment<E, T>
{
    fn fetch_random(&self, subject: [u8; 32]) -> Result<[u8; 32], RandomReadErr> {
        Ok(T::Randomness::random(&subject).0.to_fixed_bytes())
    }
}
