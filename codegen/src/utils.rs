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

use std::borrow::Borrow;

use itertools::Itertools;
use proc_macro2::TokenStream;
use quote::{
    format_ident,
    quote,
};
use syn::{
    Attribute,
    FnArg,
    Ident,
    Lit,
    Meta,
    NestedMeta,
    Pat,
    PatType,
    Type,
};

use crate::types::AttributeArgs;

#[macro_export]
macro_rules! format_err_spanned {
    ($tokens:expr, $($msg:tt)*) => {
        ::syn::Error::new_spanned(
            &$tokens,
            format_args!($($msg)*)
        )
    }
}

pub fn into_u16<T: ToString>(ident: T) -> u16 {
    let mut output = [0; 32];
    blake2b_256(ident.to_string().as_bytes(), &mut output);
    u16::from_be_bytes([output[0], output[1]])
}

pub fn into_u32<T: ToString>(ident: T) -> u32 {
    let mut output = [0; 32];
    blake2b_256(ident.to_string().as_bytes(), &mut output);
    u32::from_be_bytes([output[0], output[1], output[2], output[3]])
}

pub fn blake2b_256(input: &[u8], output: &mut [u8; 32]) {
    use ::blake2::digest::{
        consts::U32,
        Digest as _,
    };

    type Blake2b256 = blake2::Blake2b<U32>;

    let mut blake2 = Blake2b256::new();
    blake2.update(input);
    let result = blake2.finalize();
    output.copy_from_slice(&result);
}

pub trait AttributeParser<A> {
    fn split_attrs(self) -> Result<(Vec<NestedMeta>, Vec<A>), syn::Error>;
}

impl<A, I> AttributeParser<A> for I
where
    A: Borrow<Attribute>,
    I: IntoIterator<Item = A>,
{
    fn split_attrs(self) -> Result<(Vec<NestedMeta>, Vec<A>), syn::Error> {
        let (obce_attrs, other_attrs): (Vec<_>, Vec<_>) =
            self.into_iter().partition(|attr| attr.borrow().path.is_ident("obce"));

        let meta = obce_attrs
            .into_iter()
            .map(|attr| Attribute::parse_args(attr.borrow()))
            .map_ok(AttributeArgs::into_iter)
            .flatten_ok()
            .try_collect()?;

        Ok((meta, other_attrs))
    }
}

pub enum LitOrPath<'a> {
    Lit(&'a Lit),
    Path,
}

pub trait MetaUtils<'a> {
    fn find_by_name(self, name: &str) -> Option<(LitOrPath<'a>, &'a Ident)>;
}

impl<'a, I> MetaUtils<'a> for I
where
    I: IntoIterator<Item = &'a NestedMeta>,
{
    fn find_by_name(self, name: &str) -> Option<(LitOrPath<'a>, &'a Ident)> {
        self.into_iter().find_map(|attr| {
            match attr {
                NestedMeta::Meta(Meta::NameValue(value)) => {
                    if let Some(ident) = value.path.get_ident() {
                        (ident == name).then_some((LitOrPath::Lit(&value.lit), ident))
                    } else {
                        None
                    }
                }
                NestedMeta::Meta(Meta::Path(path)) => {
                    if let Some(ident) = path.get_ident() {
                        (ident == name).then_some((LitOrPath::Path, ident))
                    } else {
                        None
                    }
                }
                _ => None,
            }
        })
    }
}

pub struct InputBindings<'a> {
    bindings: Vec<&'a PatType>,
}

impl<'a> InputBindings<'a> {
    /// Iterate over "special" bindings identifiers.
    ///
    /// For example, it converts `(one: u32, two: u32)` into `(__ink_binding_0, __ink_binding_1)`.
    pub fn iter_call_params(&self) -> impl ExactSizeIterator<Item = Ident> + '_ {
        self.bindings
            .iter()
            .enumerate()
            .map(|(n, _)| format_ident!("__ink_binding_{}", n))
    }

    /// Iterate over raw bindings patterns.
    ///
    /// The provided iterator makes no conversions from the inner values stored inside [`InputBindings`].
    pub fn iter_raw_call_params(&self) -> impl ExactSizeIterator<Item = &Pat> + '_ {
        self.bindings.iter().map(|pat| &*pat.pat)
    }

    /// Create a LHS pattern from the input bindings.
    ///
    /// The returned [`TokenStream`] contains a pattern that is suitable for,
    /// for example, `scale` decoding.
    ///
    /// You can also provide an optional type, that will be used to constrain
    /// a pattern when it has `>= 1` bindings.
    pub fn lhs_pat(&self, ty: Option<Type>) -> TokenStream {
        let bindings = self.iter_call_params();
        let ty = ty.map(|val| {
            quote! {
                : #val
            }
        });

        match bindings.len() {
            0 => quote! { _ : () },
            1 => quote! { #( #bindings ),* #ty },
            _ => quote! { ( #( #bindings ),* ) #ty },
        }
    }

    /// Create a "mapping" from "special" identifiers to raw patterns.
    pub fn raw_special_mapping(&self) -> TokenStream {
        let lhs = self.bindings.iter().map(|val| &val.pat);

        let rhs = self.iter_call_params();

        quote! {
            let (#(#lhs,)*) = (#(&#rhs,)*);
        }
    }
}

impl<'a> FromIterator<&'a FnArg> for InputBindings<'a> {
    fn from_iter<T: IntoIterator<Item = &'a FnArg>>(iter: T) -> Self {
        let bindings = iter
            .into_iter()
            .filter_map(|fn_arg| {
                if let FnArg::Typed(pat) = fn_arg {
                    Some(pat)
                } else {
                    None
                }
            })
            .collect();

        Self { bindings }
    }
}

#[cfg(test)]
mod tests {
    use std::iter;

    use quote::{
        format_ident,
        quote,
    };
    use syn::{
        parse::Parser,
        parse2,
        parse_quote,
        punctuated::Punctuated,
        FnArg,
        Stmt,
        Token,
    };

    use super::InputBindings;

    #[test]
    fn special_bindings_conversion() {
        let parser = Punctuated::<FnArg, Token![,]>::parse_terminated;

        let fn_args = parser
            .parse2(quote! {
                one: u32, two: u64, three: &'a str
            })
            .unwrap();

        let input_bindings = InputBindings::from_iter(&fn_args);

        assert_eq!(
            input_bindings.iter_call_params().collect::<Vec<_>>(),
            vec![
                format_ident!("__ink_binding_0"),
                format_ident!("__ink_binding_1"),
                format_ident!("__ink_binding_2")
            ]
        );
    }

    #[test]
    fn raw_special_mapping_empty() {
        let input_bindings = InputBindings::from_iter(iter::empty());

        assert_eq!(
            parse2::<Stmt>(input_bindings.raw_special_mapping()).unwrap(),
            parse_quote! {
                let () = ();
            }
        );
    }

    #[test]
    fn raw_special_mapping_one() {
        let parser = Punctuated::<FnArg, Token![,]>::parse_terminated;

        let fn_args = parser
            .parse2(quote! {
                one: u32
            })
            .unwrap();

        let input_bindings = InputBindings::from_iter(&fn_args);

        assert_eq!(
            parse2::<Stmt>(input_bindings.raw_special_mapping()).unwrap(),
            parse_quote! {
                let (one,) = (&__ink_binding_0,);
            }
        );
    }

    #[test]
    fn raw_special_mapping_multiple() {
        let parser = Punctuated::<FnArg, Token![,]>::parse_terminated;

        let fn_args = parser
            .parse2(quote! {
                one: u32, two: u64, three: &'a str
            })
            .unwrap();

        let input_bindings = InputBindings::from_iter(&fn_args);

        assert_eq!(
            parse2::<Stmt>(input_bindings.raw_special_mapping()).unwrap(),
            parse_quote! {
                let (one, two, three,) = (&__ink_binding_0, &__ink_binding_1, &__ink_binding_2,);
            }
        );
    }
}
