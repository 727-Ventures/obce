// Copyright (c) 2012-2022 727.Ventures
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
    types::AttributeArgs,
    utils::{
        into_u16,
        into_u32,
    },
};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse2,
    Error,
    ItemTrait,
    Lit,
    Meta,
    NestedMeta,
    ReturnType,
    TraitItem,
    TraitItemMethod,
};

struct Config {
    id: u16,
}

impl Config {
    fn new(trait_item: &ItemTrait, attrs: TokenStream) -> Result<Self, Error> {
        let mut config = Self {
            id: into_u16(&trait_item.ident),
        };
        config.parse_attributes(attrs)?;
        Ok(config)
    }

    fn parse_attributes(&mut self, attrs: TokenStream) -> Result<(), Error> {
        let attrs = parse2::<AttributeArgs>(attrs.clone())
            .map_err(|error| format_err_spanned!(attrs, "unable decode attributes: {}", error,))?;

        let id = extract_attributes(attrs)?;
        if let Some(id) = id {
            self.id = id;
        }

        Ok(())
    }
}

struct MethodConfig {
    id: u16,
    input: TokenStream,
    output: TokenStream,
}

impl MethodConfig {
    fn new(method_item: &TraitItemMethod) -> Result<Self, Error> {
        let input_tys = method_item.sig.inputs.iter().filter_map(|input| {
            if let syn::FnArg::Typed(pat) = input {
                Some(pat.ty.clone())
            } else {
                None
            }
        });
        let output = match &method_item.sig.output {
            ReturnType::Default => quote! { () },
            ReturnType::Type(_, ty) => quote! { #ty },
        };

        let mut config = Self {
            id: into_u16(&method_item.sig.ident),
            input: quote! {
                ( #(#input_tys),* )
            },
            output: quote! {
                #output
            },
        };
        config.parse_attributes(method_item)?;
        Ok(config)
    }

    fn parse_attributes(&mut self, method_item: &TraitItemMethod) -> Result<(), Error> {
        for attr in method_item.attrs.iter() {
            if !attr.path.is_ident("obce") {
                continue
            }

            let attrs = parse2::<AttributeArgs>(attr.tokens.clone())
                .map_err(|error| format_err_spanned!(attr, "unable decode attributes: {}", error,))?;

            let id = extract_attributes(attrs)?;
            if let Some(id) = id {
                self.id = id;
            }
        }
        Ok(())
    }
}

pub struct ChainExtensionDefinition;

impl ChainExtensionDefinition {
    pub fn generate(attrs: TokenStream, input: TokenStream) -> Result<TokenStream, Error> {
        let mut trait_item: ItemTrait = parse2(input).unwrap();
        let config = Config::new(&trait_item, attrs)?;
        let trait_name = trait_item.ident.clone();

        let mut methods = vec![];
        for item in trait_item.items.iter() {
            if let TraitItem::Method(method) = item {
                methods.push(method.clone());
            } else {
                return Err(format_err_spanned!(
                    item,
                    "only methods are supported in the trait definition",
                ))
            }
        }

        let mut method_descriptions = vec![];
        for method in methods {
            if let Some(default) = method.default {
                return Err(format_err_spanned!(
                    default,
                    "default implementation is not supported in chains extension",
                ))
            }

            let config = MethodConfig::new(&method)?;
            let hash = into_u32(&method.sig.ident);
            let id = config.id;
            let input = config.input;
            let output = config.output;
            let (impls, types, where_clause) = trait_item.generics.split_for_impl();
            // TODO: Add check that each `id` is unique
            method_descriptions.push(quote! {
                impl #impls ::obce::codegen::MethodDescription<#hash> for dyn #trait_name #types #where_clause {
                    const ID: ::core::primitive::u16 = #id;
                    type Input = #input;
                    type Output = #output;
                }
            });
        }

        // Remove all `obce` attributes from trait's methods
        trait_item.items.iter_mut().for_each(|item| {
            if let TraitItem::Method(method) = item {
                method.attrs = method
                    .attrs
                    .clone()
                    .into_iter()
                    .filter(|attr| !attr.path.is_ident("obce"))
                    .collect();
            }
        });

        let id = config.id;
        let substrate = Self::substrate(trait_item.clone())?;
        let ink = Self::ink(trait_item.clone(), &config)?;
        let (impls, types, where_clause) = trait_item.generics.split_for_impl();

        let code = quote! {
            impl #impls ::obce::codegen::ExtensionDescription for dyn #trait_name #types #where_clause {
                const ID: ::core::primitive::u16 = #id;
            }

            #(#method_descriptions)*

            #[cfg(feature = "substrate")]
            #substrate

            #[cfg(feature = "ink")]
            #ink
        };
        Ok(code)
    }

    fn substrate(trait_item: ItemTrait) -> Result<TokenStream, Error> {
        Ok(quote! {
            #trait_item
        })
    }

    fn ink(mut trait_item: ItemTrait, trait_config: &Config) -> Result<TokenStream, Error> {
        let ext_id = (trait_config.id as u32) << 16;

        for item in trait_item.items.iter_mut() {
            if let TraitItem::Method(method) = item {
                let config = MethodConfig::new(&method)?;
                let input = config.input;
                let output = config.output;
                let func_id = config.id;

                let input_bound = parse2(quote! {
                    #input : ::scale::Encode
                })
                .map_err(|error| format_err_spanned!(method, "can't parse autogenerated encode bound {}", error))?;
                let output_bound = parse2(quote! {
                    #output : ::scale::Decode
                })
                .map_err(|error| format_err_spanned!(method, "can't parse autogenerated decode bound {}", error))?;

                if let Some(where_clause) = &mut method.sig.generics.where_clause {
                    where_clause.predicates.push(input_bound);
                    where_clause.predicates.push(output_bound);
                } else {
                    let where_clause = parse2(quote! {
                        where #input_bound, #output_bound
                    })
                    .map_err(|error| format_err_spanned!(method, "can't parse autogenerated where clause {}", error))?;

                    method.sig.generics.where_clause = Some(where_clause);
                }

                let id = ext_id | (func_id as u32);
                let input_bindings = method.sig.inputs.iter().filter_map(|input| {
                    if let syn::FnArg::Typed(pat) = input {
                        Some(pat.pat.clone())
                    } else {
                        None
                    }
                });
                method.default = Some(
                    parse2(quote! {
                        {
                            ::obce::ink::env::chain_extension::ChainExtensionMethod::build(#id)
                                .input::<#input>()
                                .output::<#output>()
                                .ignore_error_code()
                                .call(&( #(#input_bindings),* ))
                        }
                    })
                    .map_err(|error| format_err_spanned!(method, "can't parse autogenerated default {}", error))?,
                );
            }
        }

        Ok(quote! {
            #trait_item
        })
    }
}

fn extract_attributes(attrs: AttributeArgs) -> Result<Option<u16>, Error> {
    let mut id = None;
    for attr in attrs.iter() {
        match attr {
            NestedMeta::Meta(Meta::NameValue(value)) => {
                if value.path.is_ident("id") {
                    if let Lit::Int(lit_id) = &value.lit {
                        id = Some(lit_id.base10_parse::<u16>().map_err(|error| {
                            format_err_spanned!(
                                value.lit,
                                "id out of range. id must be a valid `u16` integer: {}",
                                error,
                            )
                        })?);
                    } else if let Lit::Str(lit_id) = &value.lit {
                        id = Some(into_u16(lit_id.value()));
                    } else {
                        Err(format_err_spanned!(value, "id should be integer or string"))?;
                    }
                }
            }
            _ => {
                Err(format_err_spanned!(attr, "unexpected attribute"))?;
            }
        }
    }

    Ok(id)
}
