//  Copyright 2020 Palantir Technologies, Inc.
//
//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
//! Macros for use with `zipkin`.
//!
//! You should not depend on this crate directly.
extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{parse_macro_input, Error, Expr, ImplItemFn, Lit, LitStr, Meta, Stmt, Token};

/// Wraps the execution of a function or method in a span.
///
/// Both normal and `async` methods and functions are supported. The name of the span is specified as an argument
/// to the macro attribute.
///
/// Requires the `macros` Cargo feature.
///
/// # Examples
///
/// ```ignore
/// #[zipkin::spanned(name = "shave yaks")]
/// fn shave_some_yaks(yaks: &mut [Yak]) {
///     // ...
/// }
///
/// #[zipkin::spanned(name = "asynchronously shave yaks")]
/// async fn shave_some_other_yaks(yaks: &mut [Yak]) {
///     // ...
/// }
///
/// struct Yak;
///
/// impl Yak {
///     #[zipkin::spanned(name = "shave a yak")]
///     fn shave(&mut self) {
///         // ...
///     }
///
///     #[zipkin::spanned(name = "asynchronously shave a yak")]
///     async fn shave_nonblocking(&mut self) {
///          // ...
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn spanned(args: TokenStream, item: TokenStream) -> TokenStream {
    let options = parse_macro_input!(args as Options);
    let func = parse_macro_input!(item as ImplItemFn);

    spanned_impl(options, func).unwrap_or_else(|e| e.to_compile_error().into())
}

fn spanned_impl(options: Options, mut func: ImplItemFn) -> Result<TokenStream, Error> {
    let name = &options.name;

    if func.sig.asyncness.is_some() {
        let stmts = &func.block.stmts;
        func.block.stmts = vec![
            syn::parse2(quote! {
                let __macro_impl_span = zipkin::next_span()
                    .with_name(#name)
                    .detach();
            })
            .unwrap(),
            Stmt::Expr(
                syn::parse2(quote! {
                    __macro_impl_span.bind(async move { #(#stmts)* }).await
                })
                .unwrap(),
                None,
            ),
        ];
    } else {
        let stmt = quote! {
            let __macro_impl_span = zipkin::next_span().with_name(#name);
        };
        func.block.stmts.insert(0, syn::parse2(stmt).unwrap());
    };

    Ok(func.into_token_stream().into())
}

struct Options {
    name: LitStr,
}

impl Parse for Options {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let args = Punctuated::<Meta, Token![,]>::parse_terminated(input)?;

        let mut name = None;

        for arg in args {
            let meta = match arg {
                Meta::NameValue(meta) => meta,
                _ => return Err(Error::new_spanned(&arg, "invalid attribute syntax")),
            };

            if meta.path.is_ident("name") {
                match meta.value {
                    Expr::Lit(lit) => match lit.lit {
                        Lit::Str(lit) => name = Some(lit),
                        lit => return Err(Error::new_spanned(&lit, "expected a string literal")),
                    },
                    _ => return Err(Error::new_spanned(meta, "expected `name = \"...\"`")),
                }
            } else {
                return Err(Error::new_spanned(meta.path, "unknown option"));
            }
        }

        Ok(Options {
            name: name.ok_or_else(|| Error::new(Span::call_site(), "missing `name` option"))?,
        })
    }
}
