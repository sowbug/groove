// Copyright (c) 2023 Mike Tsao. All rights reserved.

use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DataStruct, DeriveInput, Fields};

pub(crate) fn impl_nano_derive(input: TokenStream) -> TokenStream {
    TokenStream::from({
        let input = parse_macro_input!(input as DeriveInput);
        let generics = &input.generics;
        let data = &input.data;

        let struct_name = &input.ident;
        let nano_name = format_ident!("Nano{}", struct_name);
        let message_type_name = format_ident!("{}Message", struct_name);

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
                .filter(|attr| attr.path.is_ident("nano"))
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

        let mut variant_names = Vec::default();
        let mut field_names = Vec::default();
        let mut field_types = Vec::default();
        let mut getters = Vec::default();
        let mut setters = Vec::default();
        for (field_name, field_type) in sync_fields {
            variant_names.push(format_ident!(
                "{}",
                field_name.to_string().to_case(Case::Pascal),
            ));
            field_names.push(format_ident!("{}", field_name.to_string(),));
            field_types.push(format_ident!("{}", field_type));
            getters.push(format_ident!("{}", field_name.to_string(),));
            setters.push(format_ident!("set_{}", field_name.to_string(),));
        }

        let nano_struct_block = quote! {
            #[derive(Clone, Copy, Debug, Default, PartialEq)]
            pub struct #nano_name {
                #( #field_names: #field_types ),*
            }

        };
        let message_block = quote! {
            #[derive(Clone, Display, Debug, EnumCountMacro, EnumString, FromRepr, IntoStaticStr)]
            #[strum(serialize_all = "kebab-case")]
            pub enum #message_type_name {
                #struct_name ( #nano_name ),
                #( #variant_names ( #field_types ) ),*
            }
        };

        let getter_setter_block = quote! {
            impl #nano_name {
                #(
                   pub fn #getters(&self) -> #field_types { self.#field_names }
                   pub fn #setters(&mut self, #field_names: #field_types) { self.#field_names = #field_names; }
                )*
            }
        };

        let update_block = quote! {
            pub fn update(&mut self, message: #message_type_name) {
                match message {
                    #message_type_name::#struct_name(v) => *self = v,
                    #( #message_type_name::#variant_names(v) => self.#setters(v) ),*
                }
            }
        };

        let impl_block = quote! {
            pub fn message_for_name(
                &self,
                param_name: &str,
                value: groove_core::control::F32ControlValue,
            ) -> Option<#message_type_name> {
                if let Ok(message) = #message_type_name::from_str(param_name) {
                    self.parameterized_message_from_message(message, value)
                } else {
                    None
                }
            }

            pub fn message_for_index(
                &self,
                param_index: usize,
                value: groove_core::control::F32ControlValue,
            ) -> Option<#message_type_name> {
                if let Some(message) = #message_type_name::from_repr(param_index + 1) {
                    self.parameterized_message_from_message(message, value)
                } else {
                    None
                }
            }

            pub fn parameterized_message_from_message(
                &self,
                message: #message_type_name,
                value: groove_core::control::F32ControlValue,
            ) -> Option<#message_type_name> {
                match message {
                    #message_type_name::#struct_name(_) => {return None;}
                    #( #message_type_name::#variant_names(_) => {return Some(#message_type_name::#variant_names(value.into()));} )*
                }
            }
        };
        let full_message_from_nano_block = quote! {
            pub fn full_message(
                &self,
            ) -> #message_type_name {
                #message_type_name::#struct_name(*self)
            }
        };
        let full_message_from_struct_block = quote! {
            pub fn full_message(
                &self,
            ) -> #message_type_name {
                #message_type_name::#struct_name(#nano_name {
                    #( #field_names: self.#field_names, )*
                })
            }
        };
        let controllable_block = quote! {
            fn control_name_for_index(&self, index: usize) -> Option<&'static str> {
                if let Some(message) = #message_type_name::from_repr(index + 1) {
                    Some(message.into())
                } else {
                    None
                }
            }
            fn control_index_count(&self) -> usize {
                #message_type_name::COUNT - 1
            }
        };
        quote! {
            #[automatically_derived]
            #nano_struct_block
            #[automatically_derived]
            #message_block
            #[automatically_derived]
            #getter_setter_block
            #[automatically_derived]
            impl #generics #struct_name #ty_generics {
                #full_message_from_struct_block
                #impl_block
            }
            #[automatically_derived]
            impl #nano_name {
                #update_block
                #full_message_from_nano_block
                #impl_block
            }
            #[automatically_derived]
            impl groove_core::traits::Controllable for #generics #struct_name #ty_generics {
                #controllable_block
            }
            impl groove_core::traits::Controllable for #nano_name {
                #controllable_block
            }
        }
    })
}
