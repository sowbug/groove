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

pub(crate) fn impl_params_derive(input: TokenStream, primitives: &HashSet<Ident>) -> TokenStream {
    TokenStream::from({
        let input = parse_macro_input!(input as DeriveInput);
        let generics = &input.generics;
        let data = &input.data;

        let struct_name = &input.ident;
        let struct_snake_case_name = stringify!("{}", struct_name.to_string().to_case(Case::Snake));
        let params_name = format_ident!("{}Params", struct_name);

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
                .filter(|attr| attr.path.is_ident("params"))
                .collect();
            if !attrs.is_empty() {
                let is_leaf = parse_params_meta(attrs[0]);
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
        let mut setter_methods = Vec::default();
        let mut to_params_exprs = Vec::default();
        let _core_crate = format_ident!("{}", core_crate_name());
        for (field_name, field_type, is_leaf) in attr_fields {
            let field_name_pascal_case =
                format_ident!("{}", field_name.to_string().to_case(Case::Pascal),);
            variant_names.push(field_name_pascal_case.clone());
            field_names.push(field_name.clone());
            let is_leaf_or_primitive = primitives.contains(&field_type) || is_leaf;
            let field_params_type = if is_leaf_or_primitive {
                field_type.clone()
            } else {
                format_ident!("{}Params", &field_type)
            };

            getter_methods.push(if is_leaf_or_primitive {
                if field_type == "String" {
                    quote! {
                        pub fn #field_name(&self) -> &str { self.#field_name.as_ref() }
                    }
                } else {
                    quote! {
                        pub fn #field_name(&self) -> #field_params_type { self.#field_name }
                    }
                }
            } else {
                quote! {
                    pub fn #field_name(&self) -> &#field_params_type { &self.#field_name }
                }
            });
            to_params_exprs.push(if is_leaf_or_primitive {
                if field_type == "String" {
                    quote! { self.#field_name.to_string() }
                } else {
                    quote! { self.#field_name() }
                }
            } else {
                quote! { self.#field_name.to_params() }
            });
            field_types.push(field_params_type.clone());
            let setter_method_name = format_ident!("set_{}", field_name.to_string());
            if field_type == "String" {
                setter_methods.push(quote! {
                    pub fn #setter_method_name(&mut self, #field_name: &str) { self.#field_name = #field_name.to_string(); }
                });
            } else {
                setter_methods.push(quote! {
                    pub fn #setter_method_name(&mut self, #field_name: #field_params_type) { self.#field_name = #field_name; }
                });
            }
        }

        let params_struct_block = quote! {
            #[derive(Debug, Default, PartialEq,Serialize, Deserialize)]
            #[serde(rename = #struct_snake_case_name, rename_all = "kebab-case")]
            #[allow(missing_docs)]
            pub struct #params_name {
                #( pub #field_names: #field_types ),*
            }

        };

        let getter_setter_block = quote! {
            impl #params_name {
                #(
                   #getter_methods
                )*
                #(
                   #setter_methods
                )*
            }
        };

        let to_params_block = quote! {
            impl #generics #struct_name #ty_generics {
                #[allow(missing_docs)]
                pub fn to_params(&self) -> #params_name {
                    #params_name {
                        #( #field_names: #to_params_exprs, )*
                    }
                }
            }
        };

        quote! {
            #[automatically_derived]
            #params_struct_block
            #[automatically_derived]
            #getter_setter_block
            #[automatically_derived]
            #to_params_block
        }
    })
}

// Returns booleans indicating (1) whether the #[params(...)] attr indicates that
// the field type is a "leaf," meaning we shouldn't expand it to a Params type.
fn parse_params_meta(attr: &Attribute) -> bool {
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
