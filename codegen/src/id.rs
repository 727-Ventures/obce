use itertools::Itertools;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Error,
    Path,
};

use crate::{
    format_err_spanned,
    utils::into_u32,
};

pub fn generate(input: TokenStream) -> Result<TokenStream, Error> {
    let path: Path = syn::parse2(input)?;

    match path.segments.len() {
        1 => {
            let extension = path.segments.first().unwrap();

            Ok(quote! {
                <dyn #extension as ::obce::codegen::ExtensionDescription>::ID
            })
        }
        2 => {
            let (extension, method) = path.segments.iter().tuple_windows().next().unwrap();

            let method_hash = into_u32(&method.ident);

            Ok(quote! {
                <dyn #extension as ::obce::codegen::MethodDescription<#method_hash>>::ID
            })
        }
        // Support only two-segment paths to correctly identify
        // whether a user wants to get an identifier of a chain extension
        // or a chain extension method itself.
        _ => {
            Err(format_err_spanned!(
                path,
                "id macro supports only two-segment paths (ChainExtension, ChainExtension::method)"
            ))
        }
    }
}
