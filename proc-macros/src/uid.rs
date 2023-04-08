// Copyright (c) 2023 Mike Tsao. All rights reserved.

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, DeriveInput};

use crate::core_crate_name;

pub(crate) fn impl_uid_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let generics = input.generics;
    let (_impl_generics, ty_generics, _where_clause) = generics.split_for_impl();

    let core_crate = format_ident!("{}", core_crate_name());
    TokenStream::from(quote! {
        #[automatically_derived]
        impl #generics #core_crate::traits::HasUid for #name #ty_generics {
            fn uid(&self) -> usize {
                self.uid
            }

            fn set_uid(&mut self, uid: usize) {
                self.uid = uid;
            }

            fn name(&self) -> &'static str {
                stringify!(#name)
            }
        }
    })
}
