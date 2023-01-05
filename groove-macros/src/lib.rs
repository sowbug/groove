use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(Uid)]
pub fn uid_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let generics = input.generics;
    let (_impl_generics, ty_generics, _where_clause) = generics.split_for_impl();

    let expanded = quote! {
        impl #generics HasUid for #name #ty_generics {
            fn uid(&self) -> usize {
                self.uid
            }

            fn set_uid(&mut self, uid: usize) {
                self.uid = uid;
            }
        }
    };
    TokenStream::from(expanded)
}
