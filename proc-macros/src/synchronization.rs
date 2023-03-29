// Copyright (c) 2023 Mike Tsao. All rights reserved.

use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DataStruct, DeriveInput, Fields, Generics, Ident};

pub(crate) fn impl_synchronization_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = input.ident;
    let enum_name = format_ident!("{}Message", struct_name);
    TokenStream::from(parse_synchronization_data(
        &struct_name,
        &input.generics,
        &enum_name,
        &input.data,
    ))
}

fn parse_synchronization_data(
    struct_name: &Ident,
    generics: &Generics,
    enum_name: &Ident,
    data: &Data,
) -> proc_macro2::TokenStream {
    let (_impl_generics, ty_generics, _where_clause) = generics.split_for_impl();
    // Code adapted from https://blog.turbo.fish/proc-macro-error-handling/
    // Thank you!
    let fields = match data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => &fields.named,
        _ => panic!("this derive macro only works on structs with named fields"),
    };
    let sync_fields = fields.into_iter().fold(Vec::default(), |mut v, f| {
        let attrs: Vec<_> = f
            .attrs
            .iter()
            .filter(|attr| attr.path.is_ident("sync"))
            .collect();
        if !attrs.is_empty() {
            match &f.ty {
                syn::Type::Path(t) => {
                    if let Some(ident) = t.path.get_ident() {
                        v.push((f.ident.as_ref().unwrap().clone(), ident.clone()));
                    }
                }
                _ => todo!(),
            }
        }
        v
    });

    let mut enum_variant_names = Vec::default();
    let mut enum_field_names = Vec::default();
    let mut enum_field_types = Vec::default();
    let mut enum_getter_names = Vec::default();
    let mut enum_setter_names = Vec::default();
    for (field_name, field_type) in sync_fields {
        enum_variant_names.push(format_ident!(
            "{}",
            field_name.to_string().to_case(Case::Pascal),
        ));
        enum_field_names.push(format_ident!("{}", field_name.to_string(),));
        enum_field_types.push(format_ident!("{}", field_type));
        enum_getter_names.push(format_ident!("{}", field_name.to_string(),));
        enum_setter_names.push(format_ident!("set_{}", field_name.to_string(),));
    }

    let enum_block = quote! {
        #[derive(Clone, Display, Debug, EnumCountMacro, EnumString, FromRepr, IntoStaticStr)]
        #[strum(serialize_all = "kebab-case")]
        pub enum #enum_name {
            #struct_name ( #struct_name ),
            #( #enum_variant_names ( #enum_field_types ) ),*
        }
    };
    let getter_setter_block = quote! {
        impl #generics #struct_name #ty_generics {
            #(
               pub fn #enum_getter_names(&self) -> #enum_field_types { self.#enum_field_names }
               pub fn #enum_setter_names(&mut self, #enum_field_names: #enum_field_types) { self.#enum_field_names = #enum_field_names; }
            )*
        }
    };
    let impl_block = quote! {
        impl #generics #struct_name #ty_generics {
            pub fn update(&mut self, message: #enum_name) {
                match message {
                    #enum_name::#struct_name(v) => *self = v,
                    #( #enum_name::#enum_variant_names(v) => self.#enum_setter_names(v) ),*
                }
            }

            pub fn message_for_name(
                &self,
                param_name: &str,
                value: groove_core::control::F32ControlValue,
            ) -> Option<#enum_name> {
                if let Ok(message) = #enum_name::from_str(param_name) {
                    self.parameterized_message_from_message(message, value)
                } else {
                    None
                }
            }

            pub fn message_for_index(
                &self,
                param_index: usize,
                value: groove_core::control::F32ControlValue,
            ) -> Option<#enum_name> {
                if let Some(message) = #enum_name::from_repr(param_index + 1) {
                    self.parameterized_message_from_message(message, value)
                } else {
                    None
                }
            }

            pub fn full_message(
                &self,
            ) -> #enum_name {
                #enum_name::#struct_name(*self)
            }

            pub fn parameterized_message_from_message(
                &self,
                message: #enum_name,
                value: groove_core::control::F32ControlValue,
            ) -> Option<#enum_name> {
                match message {
                    #enum_name::#struct_name(_) => {return None;}
                    #( #enum_name::#enum_variant_names(_) => {return Some(#enum_name::#enum_variant_names(value.into()));} )*
                }
            }

        }
    };
    let controllable_block = quote! {
        impl groove_core::traits::Controllable for #generics #struct_name #ty_generics {
            fn control_name_for_index(&self, index: usize) -> Option<&'static str> {
                if let Some(message) = #enum_name::from_repr(index + 1) {
                    Some(message.into())
                } else {
                    None
                }
            }
            fn control_index_count(&self) -> usize {
                #enum_name::COUNT - 1
            }
        }
    };
    quote! {
        #[automatically_derived]
        #enum_block
        #[automatically_derived]
        #getter_setter_block
        #[automatically_derived]
        #impl_block
        #[automatically_derived]
        #controllable_block
    }
}
