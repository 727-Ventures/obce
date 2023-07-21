use proc_macro2::TokenStream;
use quote::{
    format_ident,
    quote,
};
use syn::{
    parse2,
    parse_quote,
    Error,
    ImplItem,
    ItemImpl,
    ItemTrait,
    TraitItem,
    TraitItemMethod,
};

use crate::{
    format_err_spanned,
    utils::{
        into_u32,
        InputBindings,
    },
};

pub fn generate(_: TokenStream, input: TokenStream) -> Result<TokenStream, Error> {
    let mut impl_item: ItemImpl = parse2(input)?;

    let Some((_, trait_name, _)) = impl_item.trait_ else {
        return Err(format_err_spanned!(
            impl_item,
            "impl marked as mocked should have a trait present"
        ))
    };
    let item = impl_item.self_ty;

    let (impls, types, where_clause) = impl_item.generics.split_for_impl();

    // We assume that every single item is a method.
    let methods = impl_item
        .items
        .iter_mut()
        .filter_map(|item| {
            if let ImplItem::Method(method_item) = item {
                Some(method_item)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    let mut mock_trait: ItemTrait = parse_quote! {
        trait MockTrait {}
    };

    mock_trait.generics = impl_item.generics.clone();
    mock_trait.items = methods
        .iter()
        .map(|method| (**method).clone())
        .map(|val| {
            TraitItem::Method(TraitItemMethod {
                attrs: val.attrs,
                sig: val.sig,
                default: None,
                semi_token: None,
            })
        })
        .collect();

    let mut mock_impl: ItemImpl = parse_quote! {
        impl MockTrait #types for #item {}
    };

    mock_impl.generics = impl_item.generics.clone();
    mock_impl.items = methods
        .iter()
        .map(|method| (**method).clone())
        .map(ImplItem::Method)
        .collect();

    let proxies = methods.iter()
        .map(|method| {
            let hash = into_u32(&method.sig.ident);

            let method_name = &method.sig.ident;
            let proxy_name = format_ident!("ProxyFor{}", hash);
            let proxy_where_clause = if let Some(mut where_clause) = where_clause.cloned() {
                where_clause.predicates.push(parse_quote! {
                    dyn #trait_name: ::obce::codegen::ExtensionDescription,
                });
                where_clause.predicates.push(parse_quote! {
                    <dyn #trait_name as ::obce::codegen::MethodDescription<#hash>>::Output: ::scale::Encode,
                });
                where_clause.predicates.push(parse_quote! {
                    <dyn #trait_name as ::obce::codegen::MethodDescription<#hash>>::Input: ::scale::Decode
                });
                where_clause
            } else {
                parse_quote! {
                    where
                        dyn #trait_name: ::obce::codegen::ExtensionDescription,
                        <dyn #trait_name as ::obce::codegen::MethodDescription<#hash>>::Output: ::scale::Encode,
                        <dyn #trait_name as ::obce::codegen::MethodDescription<#hash>>::Input: ::scale::Decode
                }
            };

            let input_bindings = InputBindings::from_iter(&method.sig.inputs);
            let lhs_pat = input_bindings.lhs_pat(Some(parse_quote! {
                <dyn #trait_name as ::obce::codegen::MethodDescription<#hash>>::Input
            }));
            let call_params = input_bindings.iter_call_params();

            quote! {
                struct #proxy_name #types (::std::rc::Rc<::std::cell::RefCell<#item>>);

                impl #impls ::obce::ink_lang::env::test::ChainExtension for #proxy_name #types #proxy_where_clause {
                    fn func_id(&self) -> u32 {
                        let trait_id = <dyn #trait_name as ::obce::codegen::ExtensionDescription>::ID;
                        let func_id = <dyn #trait_name as ::obce::codegen::MethodDescription<#hash>>::ID;
                        (trait_id as u32) << 16 | (func_id as u32)
                    }

                    fn call(&mut self, mut input: &[u8], output: &mut Vec<u8>) -> u32 {
                        let context = &mut *self.0.borrow_mut();

                        let bytes: Vec<u8> = ::scale::Decode::decode(&mut &input[..])
                            .unwrap();

                        let #lhs_pat = ::scale::Decode::decode(&mut &bytes[..])
                            .unwrap();

                        #[allow(clippy::unnecessary_mut_passed)]
                        let call_output: <dyn #trait_name as ::obce::codegen::MethodDescription<#hash>>::Output = <#item as MockTrait #types>::#method_name(
                            context
                            #(, #call_params)*
                        );

                        ::scale::Encode::encode_to(&call_output, output);

                        0
                    }
                }

                ::obce::ink_lang::env::test::register_chain_extension(#proxy_name(wrapped_context.clone()));
            }
        });

    Ok(quote! {
        pub fn register_chain_extensions #types (ctx: #item) {
            #[allow(unused_variables)]
            let wrapped_context = ::std::rc::Rc::new(::std::cell::RefCell::new(ctx));

            #mock_trait

            #mock_impl

            #(#proxies)*
        }
    })
}
