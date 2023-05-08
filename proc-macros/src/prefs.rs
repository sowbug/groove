// Copyright (c) 2023 Mike Tsao. All rights reserved.

use std::collections::HashSet;

use crate::core_crate_name;
use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, Attribute, Data, DataStruct, DeriveInput, Fields, Ident, Lit, Meta,
    NestedMeta,
};

pub(crate) fn impl_prefs_derive(input: TokenStream, primitives: &HashSet<Ident>) -> TokenStream {
    TokenStream::from({
        let input = parse_macro_input!(input as DeriveInput);
        let generics = &input.generics;
        let data = &input.data;

        let struct_name = &input.ident;
        let struct_snake_case_name = stringify!("{}", struct_name.to_string().to_case(Case::Snake));
        let prefs_name = format_ident!("{}Prefs", struct_name);

        let (_impl_generics, _ty_generics, _where_clause) = generics.split_for_impl();
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
                .filter(|attr| attr.path.is_ident("prefs"))
                .collect();
            if !attrs.is_empty() {
                let is_leaf = parse_prefs_meta(attrs[0]);
                match &f.ty {
                    syn::Type::Path(t) => {
                        if let Some(ident) = t.path.get_ident() {
                            v.push((f.ident.as_ref().unwrap().clone(), ident.clone(), is_leaf));
                        }
                    }
                    _ => todo!(),
                }
            }
            v
        });

        let mut field_names = Vec::default();
        let mut field_types = Vec::default();
        let mut variant_names = Vec::default();
        let mut getter_methods = Vec::default();
        let mut setters = Vec::default();
        let _core_crate = format_ident!("{}", core_crate_name());
        for (field_name, field_type, is_leaf) in attr_fields {
            let field_name_pascal_case =
                format_ident!("{}", field_name.to_string().to_case(Case::Pascal),);
            variant_names.push(field_name_pascal_case.clone());
            field_names.push(field_name.clone());
            let is_leaf_or_primitive = primitives.contains(&field_type) || is_leaf;
            let field_prefs_type = if is_leaf_or_primitive {
                field_type.clone()
            } else {
                format_ident!("{}Prefs", &field_type)
            };

            getter_methods.push(if is_leaf_or_primitive {
                quote! {
                    pub fn #field_name(&self) -> #field_prefs_type { self.#field_name }
                }
            } else {
                quote! {
                    pub fn #field_name(&self) -> &#field_prefs_type { &self.#field_name }
                }
            });
            field_types.push(field_prefs_type);
            setters.push(format_ident!("set_{}", field_name.to_string(),));
        }

        let prefs_struct_block = quote! {
            #[derive(Clone, Debug, Default, PartialEq)]
            #[cfg_attr(
                feature = "serialization",
                derive(Serialize, Deserialize),
                serde(rename = #struct_snake_case_name, rename_all = "kebab-case")
            )]
            pub struct #prefs_name {
                #( pub #field_names: #field_types ),*
            }

        };

        let getter_setter_block = quote! {
            impl #prefs_name {
                #(
                   #getter_methods
                )*
                #(
                   pub fn #setters(&mut self, #field_names: #field_types) { self.#field_names = #field_names.clone(); }
                )*
            }
        };

        quote! {
            #[automatically_derived]
            #prefs_struct_block
            #[automatically_derived]
            #getter_setter_block
        }
    })
}

// Returns booleans indicating (1) whether the #[prefs(...)] attr indicates that
// the field type is a "leaf," meaning we shouldn't expand it to a Prefs type.
fn parse_prefs_meta(attr: &Attribute) -> bool {
    let mut is_leaf = false;
    if let Ok(meta) = attr.parse_meta() {
        let meta_list = match meta {
            Meta::List(list) => list,
            _ => {
                return is_leaf;
            }
        };

        let punctuated = match meta_list.nested.len() {
            0 => return is_leaf,
            _ => &meta_list.nested,
        };

        punctuated.iter().for_each(|nested| {
            if let NestedMeta::Meta(Meta::NameValue(name_value)) = nested {
                if name_value.path.is_ident("leaf") {
                    is_leaf = get_bool_from_lit(name_value);
                } else {
                    // Unsupported attribute; ignore
                }
            } else {
                // Unexpected stuff; ignore
            }
        });
    }
    is_leaf
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
