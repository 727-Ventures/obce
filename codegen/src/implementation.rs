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

use crate::{
    format_err_spanned,
    utils::{
        into_u32,
        AttributeParser,
        InputBindings,
        LitOrPath,
        MetaUtils,
    },
};
use itertools::Itertools;
use proc_macro2::{
    Ident,
    TokenStream,
};
use quote::{
    format_ident,
    quote,
    ToTokens,
};
use syn::{
    parse::Parser,
    parse2,
    parse_quote,
    parse_str,
    punctuated::Punctuated,
    Error,
    Expr,
    GenericArgument,
    Generics,
    ImplItem,
    ItemImpl,
    Lit,
    Meta,
    NestedMeta,
    Path,
    PathArguments,
    Token,
    Type,
};
use tuple::Map;

pub fn generate(_attrs: TokenStream, input: TokenStream) -> Result<TokenStream, Error> {
    let impl_item: ItemImpl = parse2(input).unwrap();

    let mut original_implementation = impl_item.clone();

    let method_items = original_implementation.items.iter_mut().filter_map(|item| {
        if let ImplItem::Method(method_item) = item {
            Some(method_item)
        } else {
            None
        }
    });

    for method_item in method_items {
        let (_, other_attrs) = method_item.attrs.iter().cloned().split_attrs()?;

        method_item.attrs = other_attrs;
    }

    let chain_extension = chain_extension_trait_impl(impl_item)?;

    Ok(quote! {
        // Implementation of the trait for `ExtensionContext` with main logic.
        #original_implementation

        // Implementation of `ChainExtension` from `contract-pallet`
        #chain_extension
    })
}

#[allow(non_snake_case)]
fn chain_extension_trait_impl(mut impl_item: ItemImpl) -> Result<TokenStream, Error> {
    let context = ExtensionContext::try_from(&impl_item)?;

    let namespace = quote! { ::obce::substrate::pallet_contracts::chain_extension:: };

    let T = context.substrate;
    let E = context.env;
    let Env = context.obce_env;
    let extension = context.extension;

    let mut callable_generics = impl_item.generics.clone();
    callable_generics = filter_generics(callable_generics, &context.lifetime1);
    let (callable_impls, _, callable_where) = callable_generics.split_for_impl();

    let mut main_generics = impl_item.generics.clone();
    main_generics = filter_generics(main_generics, &context.lifetime1);
    main_generics = filter_generics(main_generics, &E);
    main_generics = filter_generics(main_generics, &Env);
    let (main_impls, _, main_where) = main_generics.split_for_impl();

    let mut call_generics = impl_item.generics.clone();
    call_generics = filter_generics(call_generics, &context.lifetime1);
    call_generics = filter_generics(call_generics, &Env);

    // User is not required to use `Ext` trait for testing, so we automatically
    // add `Ext` bound when generating "production" code.
    if let Some(where_clause) = &mut call_generics.where_clause {
        where_clause.predicates.push(parse_quote! {
            #E: #namespace Ext<T = #T>
        });
    } else {
        call_generics.where_clause = Some(parse_quote! {
            where #E: #namespace Ext<T = #T>
        });
    }

    let (_, _, call_where) = call_generics.split_for_impl();

    let trait_;
    let dyn_trait;
    if let Some((_, path, _)) = impl_item.trait_ {
        trait_ = path.clone();
        dyn_trait = quote! { dyn #path };
    } else {
        return Err(format_err_spanned!(impl_item, "expected impl trait block",))
    }

    let methods: Vec<_> = impl_item
        .items
        .iter_mut()
        .filter_map(|item| {
            if let ImplItem::Method(method) = item {
                Some(method)
            } else {
                None
            }
        })
        .map(|method| {
            let (obce_attrs, other_attrs) = method.attrs.iter().cloned().split_attrs()?;

            method.attrs = other_attrs;

            let hash = into_u32(&method.sig.ident);
            let method_name = &method.sig.ident;

            let input_bindings = InputBindings::from_iter(&method.sig.inputs);
            let lhs_pat = input_bindings.lhs_pat(None);
            let call_params = input_bindings.iter_call_params();

            let (weight_tokens, pre_charge) = handle_weight_attribute(&input_bindings, obce_attrs.iter())?;
            let ret_val_tokens = handle_ret_val_attribute(obce_attrs.iter());

            let (read_with_charge, pre_charge_arg) = if pre_charge {
                (
                    quote! {
                        let pre_charged = #weight_tokens;
                        let #lhs_pat = env.read_as_unbounded(len)?;
                    },
                    quote! {
                        Some(pre_charged)
                    },
                )
            } else {
                (
                    quote! {
                        let #lhs_pat = env.read_as_unbounded(len)?;
                        #weight_tokens;
                    },
                    quote! {
                        None
                    },
                )
            };

            Result::<_, Error>::Ok(quote! {
                <#dyn_trait as ::obce::codegen::MethodDescription<#hash>>::ID => {
                    #read_with_charge
                    let mut context = ::obce::substrate::ExtensionContext::new(self, env, #pre_charge_arg);
                    #[allow(clippy::unnecessary_mut_passed)]
                    let result = <_ as #trait_>::#method_name(
                        &mut context
                        #(, #call_params)*
                    );

                    // If result is `Result` and `Err` is critical, return from the `call`.
                    // Otherwise, try to convert result to RetVal, and return it or encode the result into the buffer.
                    let result = ::obce::to_critical_error!(result)?;
                    #ret_val_tokens
                    <_ as ::scale::Encode>::using_encoded(&result, |w| context.env.write(w, true, None))?;
                },
            })
        })
        .try_collect()?;

    Ok(quote! {
        impl #callable_impls ::obce::substrate::CallableChainExtension<#E, #T, #Env> for #extension
            #callable_where
        {
            fn call(&mut self, mut env: #Env) -> ::core::result::Result<
                #namespace RetVal,
                ::obce::substrate::CriticalError
            > {
                let len = env.in_len();

                match env.func_id() {
                    #(#methods)*
                    _ => ::core::result::Result::Err(::obce::substrate::CriticalError::Other(
                        "InvalidFunctionId"
                    ))?,
                };

                Ok(#namespace RetVal::Converging(0))
            }
        }

        impl #main_impls #namespace ChainExtension<#T> for #extension #main_where {
            fn call<#E>(&mut self, env: #namespace Environment<#E, #namespace InitState>)
                -> ::core::result::Result<#namespace RetVal, ::obce::substrate::CriticalError>
                #call_where
            {
                <#extension as ::obce::substrate::CallableChainExtension<#E, #T, _>>::call(
                    self, env.buf_in_buf_out()
                )
            }
        }

        impl #main_impls #namespace RegisteredChainExtension<#T> for #extension #main_where {
            const ID: ::core::primitive::u16 = <#dyn_trait as ::obce::codegen::ExtensionDescription>::ID;
        }
    })
}

struct ExtensionContext {
    // Lifetime `'a`
    lifetime1: GenericArgument,
    // Generic `E`
    env: GenericArgument,
    // Generic `T`
    substrate: GenericArgument,
    // Generic `Env`
    obce_env: GenericArgument,
    // Generic `Extension`
    extension: GenericArgument,
}

impl TryFrom<&ItemImpl> for ExtensionContext {
    type Error = Error;

    fn try_from(impl_item: &ItemImpl) -> Result<Self, Self::Error> {
        let Type::Path(path) = impl_item.self_ty.as_ref() else {
            return Err(format_err_spanned!(impl_item, "the type should be `ExtensionContext`"))
        };

        let Some(extension) = path.path.segments.last() else {
            return Err(format_err_spanned!(path, "the type should be `ExtensionContext`"))
        };

        let PathArguments::AngleBracketed(generic_args) = &extension.arguments else {
            return Err(format_err_spanned!(
                path,
                "`ExtensionContext` should have 5 generics as `<'a, E, T, Env, Extension>`"
            ))
        };

        let (lifetime1, env, substrate, obce_env, extension) =
            generic_args.args.iter().cloned().tuples().exactly_one().map_err(|_| {
                format_err_spanned!(
                    generic_args,
                    "`ExtensionContext` should have 5 generics as `<'a, E, T, Env, Extension>`"
                )
            })?;

        Ok(ExtensionContext {
            lifetime1,
            env,
            substrate,
            obce_env,
            extension,
        })
    }
}

fn filter_generics(mut generics: Generics, filter: &GenericArgument) -> Generics {
    let filter: Vec<_> = filter
        .to_token_stream()
        .into_iter()
        .map(|token| token.to_string())
        .collect();
    generics.params = generics
        .params
        .clone()
        .into_iter()
        .filter(|param| {
            let param: Vec<_> = param
                .to_token_stream()
                .into_iter()
                .map(|token| token.to_string())
                .collect();
            !is_subsequence(&param, &filter)
        })
        .collect();

    if let Some(where_clause) = &mut generics.where_clause {
        where_clause.predicates = where_clause
            .predicates
            .clone()
            .into_iter()
            .filter(|predicate| {
                let predicate: Vec<_> = predicate
                    .to_token_stream()
                    .into_iter()
                    .map(|token| token.to_string())
                    .collect();
                !is_subsequence(&predicate, &filter)
            })
            .collect();
    }

    generics
}

fn is_subsequence<T: PartialEq + core::fmt::Debug>(src: &[T], search: &[T]) -> bool {
    if search.len() > src.len() {
        return false
    }

    for i in 0..(src.len() - search.len() + 1) {
        if &src[i..(i + search.len())] == search {
            return true
        }
    }
    false
}

fn handle_ret_val_attribute<'a, I: IntoIterator<Item = &'a NestedMeta>>(iter: I) -> Option<TokenStream> {
    let should_handle = iter.into_iter().any(|attr| {
        if let NestedMeta::Meta(Meta::Path(path)) = attr {
            if let Some(ident) = path.get_ident() {
                return ident == "ret_val"
            }
        }

        false
    });

    should_handle.then(|| {
        quote! {
            if let Err(error) = result {
                if let Ok(ret_val) = error.try_into() {
                    return Ok(ret_val)
                }
            }
        }
    })
}

fn handle_weight_attribute<'a, I: IntoIterator<Item = &'a NestedMeta>>(
    input_bindings: &InputBindings,
    iter: I,
) -> Result<(Option<TokenStream>, bool), Error> {
    let weight_params = iter.into_iter().find_map(|attr| {
        let NestedMeta::Meta(Meta::List(list)) = attr else {
            return None
        };

        let Some(ident) = list.path.get_ident() else {
            return None
        };

        (ident == "weight").then_some((&list.nested, ident))
    });

    if let Some((weight_params, weight_ident)) = weight_params {
        match weight_params.iter().find_by_name("dispatch") {
            Some((LitOrPath::Lit(Lit::Str(dispatch_path)), ident)) => {
                let args = match weight_params.iter().find_by_name("args") {
                    Some((LitOrPath::Lit(Lit::Str(args)), _)) => Some(args.value()),
                    None => None,
                    Some((_, ident)) => {
                        return Err(format_err_spanned!(
                            ident,
                            "`args` attribute should contain a comma-separated expression list"
                        ))
                    }
                };

                return Ok((
                    Some(handle_dispatch_weight(
                        ident,
                        input_bindings,
                        &dispatch_path.value(),
                        args.as_deref(),
                    )?),
                    false,
                ))
            }
            Some((_, ident)) => {
                return Err(format_err_spanned!(
                    ident,
                    "`dispatch` attribute should contain a pallet method path"
                ))
            }
            None => {}
        };

        match weight_params.iter().find_by_name("expr") {
            Some((LitOrPath::Lit(Lit::Str(expr)), _)) => {
                let pre_charge = matches!(
                    weight_params.iter().find_by_name("pre_charge"),
                    Some((LitOrPath::Path, _))
                );

                return Ok((
                    Some(handle_expr_weight(input_bindings, &expr.value(), pre_charge)?),
                    pre_charge,
                ))
            }
            Some((_, ident)) => {
                return Err(format_err_spanned!(
                    ident,
                    "`expr` attribute should contain an expression that returns `Weight`"
                ))
            }
            None => {}
        }

        Err(format_err_spanned!(
            weight_ident,
            r#"either "dispatch" or "expr" attributes are expected"#
        ))
    } else {
        Ok((None, false))
    }
}

fn handle_expr_weight(input_bindings: &InputBindings, expr: &str, pre_charge: bool) -> Result<TokenStream, Error> {
    let expr = parse_str::<Expr>(expr)?;

    let raw_map = if pre_charge {
        quote! {}
    } else {
        input_bindings.raw_special_mapping()
    };

    Ok(quote! {{
        #[allow(unused_variables)]
        #raw_map
        env.charge_weight(#expr)?
    }})
}

fn handle_dispatch_weight(
    ident: &Ident,
    input_bindings: &InputBindings,
    dispatch_path: &str,
    args: Option<&str>,
) -> Result<TokenStream, Error> {
    let segments = parse_str::<Path>(dispatch_path)?.segments.into_iter();
    let segments_len = segments.len();

    if segments_len < 3 {
        return Err(format_err_spanned!(
            ident,
            "dispatch path should contain at least three segments"
        ))
    }

    let (pallet_ns, _, method_name) = segments
        .enumerate()
        .group_by(|(idx, _)| if *idx < segments_len - 2 { 0 } else { *idx })
        .into_iter()
        .map(|(_, group)| group.map(|(_, segment)| segment))
        .next_tuple::<(_, _, _)>()
        .unwrap()
        .map(Punctuated::<_, Token![::]>::from_iter);

    let dispatch_args = if let Some(args) = args {
        let parser = Punctuated::<Expr, Token![,]>::parse_terminated;
        parser.parse_str(args)?.to_token_stream()
    } else {
        let raw_call_params = input_bindings.iter_raw_call_params();

        // If no args were provided try to call the pallet method using default outer args.
        quote! {
            #(*#raw_call_params,)*
        }
    };

    let call_variant_name = format_ident!("new_call_variant_{}", method_name.last().unwrap().ident);

    let raw_map = input_bindings.raw_special_mapping();

    Ok(quote! {{
        #[allow(unused_variables)]
        #raw_map
        let __call_variant = &#pallet_ns ::Call::<T>::#call_variant_name(#dispatch_args);
        let __dispatch_info = <#pallet_ns ::Call<T> as ::obce::substrate::frame_support::dispatch::GetDispatchInfo>::get_dispatch_info(__call_variant);
        env.charge_weight(__dispatch_info.weight)?
    }})
}
