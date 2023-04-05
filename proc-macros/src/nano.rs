// Copyright (c) 2023 Mike Tsao. All rights reserved.

use crate::core_crate_name;
use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, Attribute, Data, DataStruct, DeriveInput, Fields, Lit, Meta, NestedMeta,
};

pub(crate) fn impl_nano_derive(input: TokenStream) -> TokenStream {
    TokenStream::from({
        let input = parse_macro_input!(input as DeriveInput);
        let generics = &input.generics;
        let data = &input.data;

        let struct_name = &input.ident;
        let struct_snake_case_name = stringify!("{}", struct_name.to_string().to_case(Case::Snake));
        let nano_name = format_ident!("{}Nano", struct_name);
        let message_type_name = format_ident!("{}Message", struct_name);
        let unit_only_enum_name = format_ident!("{}UnitOnly", struct_name);

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
        let attr_fields = fields.into_iter().fold(Vec::default(), |mut v, f| {
            let attrs: Vec<_> = f
                .attrs
                .iter()
                .filter(|attr| attr.path.is_ident("nano"))
                .collect();
            if !attrs.is_empty() {
                let (should_control, is_no_copy) = parse_nano_meta(attrs[0]);
                match &f.ty {
                    syn::Type::Path(t) => {
                        if let Some(ident) = t.path.get_ident() {
                            v.push((
                                f.ident.as_ref().unwrap().clone(),
                                ident.clone(),
                                should_control,
                                is_no_copy,
                            ));
                        }
                    }
                    _ => todo!(),
                }
            }
            v
        });

        // co = "control-only" meaning fields that don't have the control=false attribute
        // nc = "non-copy" meaning the field represents a struct that is not #[derive(Copy)]
        let mut co_field_names = Vec::default();
        let mut co_field_types = Vec::default();
        let mut co_variant_names = Vec::default();
        let mut field_names = Vec::default();
        let mut field_types = Vec::default();
        let mut variant_names = Vec::default();
        let mut getter_methods = Vec::default();
        let mut nc_getter_methods = Vec::default();
        let mut setters = Vec::default();
        let core_crate = format_ident!("{}", core_crate_name());
        for (field_name, field_type, should_control, is_no_copy) in attr_fields {
            let field_name_pascal_case =
                format_ident!("{}", field_name.to_string().to_case(Case::Pascal),);
            variant_names.push(field_name_pascal_case.clone());
            if should_control {
                co_variant_names.push(field_name_pascal_case);
                co_field_names.push(field_name.clone());
                co_field_types.push(field_type.clone());
            }
            field_names.push(field_name.clone());
            field_types.push(field_type.clone());

            // If the field is annotated copy=false, then we generate an
            // immutable borrow getter rather than a simple getter.
            if is_no_copy {
                nc_getter_methods.push(quote! {
                    pub fn #field_name(&self) -> &#field_type { &self.#field_name }
                });
            } else {
                getter_methods.push(quote! {
                    pub fn #field_name(&self) -> #field_type { self.#field_name }
                });
            }
            setters.push(format_ident!("set_{}", field_name.to_string(),));
        }

        let nano_struct_block = quote! {
            #[derive(Clone, Debug, Default, PartialEq)]
            #[cfg_attr(
                feature = "serialization",
                derive(Serialize, Deserialize),
                serde(rename = #struct_snake_case_name, rename_all = "kebab-case")
            )]
            pub struct #nano_name {
                #( pub #field_names: #field_types ),*
            }

        };

        let message_block = quote! {
            #[derive(Clone, Display, Debug, PartialEq)]
            pub enum #message_type_name {
                #struct_name ( #nano_name ),
                #( #variant_names ( #field_types ) ),*
            }
        };

        // https://doc.rust-lang.org/reference/items/enumerations.html
        //
        // I need a way to convert enum names into indexes, and I lost the easy
        // way when my enums started carrying more complex structs, thus no
        // longer being "unit-only enums." Rather than fight this, I am making a
        // separate unit-only enum that does what I want!
        let unit_enum_block = quote! {
            #[derive(Debug, EnumCountMacro, EnumString, FromRepr, IntoStaticStr)]
            #[strum(serialize_all = "kebab-case")]
            pub enum #unit_only_enum_name {
                #( #co_variant_names ),*
            }
        };

        let getter_setter_block = quote! {
            impl #nano_name {
                #(
                   #getter_methods
                )*
                #(
                   #nc_getter_methods
                )*
                #(
                   pub fn #setters(&mut self, #field_names: #field_types) { self.#field_names = #field_names.clone(); }
                )*
            }
        };

        let nano_update_block = quote! {
            pub fn update(&mut self, message: #message_type_name) {
                match message {
                    #message_type_name::#struct_name(v) => *self = v,
                    #( #message_type_name::#variant_names(#field_names) => self.#setters(#field_names) ),*
                }
            }
        };

        let update_block = quote! {
            pub fn derived_update(&mut self, message: #message_type_name) {
                match message {
                    #message_type_name::#struct_name(e) => panic!("You must handle the full struct message yourself, because we haven't decided how to handle the sample_rate case."),
                    #( #message_type_name::#variant_names(#field_names) => self.#setters(#field_names) ),*
                }
            }
        };

        let impl_block = quote! {
            pub fn message_for_name(
                &self,
                param_name: &str,
                value: #core_crate::control::F32ControlValue,
            ) -> Option<#message_type_name> {
                if let Ok(unit_enum) = #unit_only_enum_name::from_str(param_name) {
                    self.parameterized_message_from_unit_enum(unit_enum, value)
                } else {
                    None
                }
            }

            pub fn message_for_index(
                &self,
                param_index: usize,
                value: #core_crate::control::F32ControlValue,
            ) -> Option<#message_type_name> {
                if let Some(unit_enum) = #unit_only_enum_name::from_repr(param_index) {
                    self.parameterized_message_from_unit_enum(unit_enum, value)
                } else {
                    None
                }
            }

            pub fn parameterized_message_from_unit_enum(
                &self,
                unit_enum: #unit_only_enum_name,
                value: #core_crate::control::F32ControlValue,
            ) -> Option<#message_type_name> {
                match unit_enum {
                    #( #unit_only_enum_name::#co_variant_names => {return Some(#message_type_name::#co_variant_names(value.into()));} )*
                }
            }
        };
        let full_message_from_nano_block = quote! {
            pub fn full_message(
                &self,
            ) -> #message_type_name {
                #message_type_name::#struct_name(self.clone())
            }
        };
        let full_message_from_struct_block = quote! {
            pub fn full_message(
                &self,
            ) -> #message_type_name {
                #message_type_name::#struct_name(#nano_name {
                    #( #field_names: self.#field_names.clone(), )*
                })
            }
        };
        let controllable_block = quote! {
            fn control_name_for_index(&self, index: usize) -> Option<&'static str> {
                if let Some(message) = #unit_only_enum_name::from_repr(index) {
                    Some(message.into())
                } else {
                    None
                }
            }
            fn control_index_for_name(&self, name: &str) -> usize {
                if let Ok(param) = #unit_only_enum_name::from_str(name) {
                    param as usize
                } else {
                    eprintln!("Unrecognized control param name: {}", name);
                    usize::MAX
                }
            }
            fn control_index_count(&self) -> usize {
                #unit_only_enum_name::COUNT
            }
        };
        quote! {
            #[automatically_derived]
            #nano_struct_block
            #[automatically_derived]
            #message_block
            #[automatically_derived]
            #unit_enum_block
            #[automatically_derived]
            #getter_setter_block
            #[automatically_derived]
            impl #generics #struct_name #ty_generics {
                #full_message_from_struct_block
                #impl_block
                #update_block
            }
            #[automatically_derived]
            impl #nano_name {
                #nano_update_block
                #full_message_from_nano_block
                #impl_block
            }
            #[automatically_derived]
            impl #generics #core_crate::traits::Controllable for #struct_name #ty_generics {
                #controllable_block
            }
            impl #core_crate::traits::Controllable for #nano_name {
                #controllable_block
            }
        }
    })
}

// Returns booleans indicating (1) whether the #[nano(...)] attr indicates that
// it's OK to emit control infrastructure, (2) whether the structure is
// designated "no_copy," which means we have to handle it a little differently.
fn parse_nano_meta(attr: &Attribute) -> (bool, bool) {
    let mut should_control = true;
    let mut is_no_copy = false;
    if let Ok(meta) = attr.parse_meta() {
        let meta_list = match meta {
            Meta::List(list) => list,
            _ => {
                return (should_control, is_no_copy);
            }
        };

        let punctuated = match meta_list.nested.len() {
            0 => return (should_control, is_no_copy),
            _ => &meta_list.nested,
        };

        punctuated.iter().for_each(|nested| {
            if let NestedMeta::Meta(Meta::NameValue(name_value)) = nested {
                if name_value.path.is_ident("control") {
                    should_control = get_bool_from_lit(name_value);
                } else if name_value.path.is_ident("no_copy") {
                    is_no_copy = get_bool_from_lit(name_value);
                } else {
                    // Unsupported attribute; ignore
                }
            } else {
                // Unexpected stuff; ignore
            }
        });
    }
    (should_control, is_no_copy)
}

fn get_bool_from_lit(name_value: &syn::MetaNameValue) -> bool {
    match &name_value.lit {
        Lit::Bool(bool_val) => {
            return bool_val.value();
        }
        _ => {}
    }
    false
}
