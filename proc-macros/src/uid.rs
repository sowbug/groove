// Copyright (c) 2023 Mike Tsao. All rights reserved.

use crate::core_crate_name;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, DeriveInput};

pub(crate) fn impl_uid_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let generics = input.generics;
    let (_impl_generics, ty_generics, _where_clause) = generics.split_for_impl();

    let core_crate = format_ident!("{}", core_crate_name());
    let core_crate_ensnare_migration = format_ident!("{}", "ensnare"); // TODO: clean up when migration is complete
    TokenStream::from(quote! {
        #[automatically_derived]
        impl #generics #core_crate::traits::HasUid for #name #ty_generics {
            fn uid(&self) -> #core_crate_ensnare_migration::uid::Uid {
                self.uid
            }

            fn set_uid(&mut self, uid: #core_crate_ensnare_migration::uid::Uid) {
                self.uid = uid;
            }

            fn name(&self) -> &'static str {
                stringify!(#name)
            }
        }
    })
}
