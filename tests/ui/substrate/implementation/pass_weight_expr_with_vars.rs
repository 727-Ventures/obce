use obce::substrate::{
    sp_weights::Weight,
    frame_system::Config as SysConfig,
    pallet_contracts::Config as ContractConfig,
    sp_runtime::traits::StaticLookup,
    ChainExtensionEnvironment,
    ExtensionContext
};

pub struct ChainExtension;

#[obce::definition]
pub trait ChainExtensionDefinition {
    fn extension_method(&mut self, val: u64);
}

#[obce::implementation]
impl<'a, E, T, Env> ChainExtensionDefinition for ExtensionContext<'a, E, T, Env, ChainExtension>
where
    T: SysConfig + ContractConfig,
    <<T as SysConfig>::Lookup as StaticLookup>::Source: From<<T as SysConfig>::AccountId>,
    Env: ChainExtensionEnvironment<E, T>,
{
    #[obce(weight(expr = "Weight::from_parts(*_val, 0)"))]
    fn extension_method(&mut self, _val: u64) {}
}

fn main() {}
