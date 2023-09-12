// Copyright (c) 2023 Mike Tsao. All rights reserved.

use crate::core_crate_name;
use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use std::collections::HashSet;
use syn::{
    parse_macro_input, Attribute, Data, DataStruct, DeriveInput, Fields, Ident, Lit, Meta,
    NestedMeta,
};

// TODO: see
// https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=03943d1dfbf41bd63878bfccb1c64670
// for an intriguing bit of code. Came from
// https://users.rust-lang.org/t/is-implementing-a-derive-macro-for-converting-nested-structs-to-flat-structs-possible/65839/3

pub(crate) fn impl_control_derive(input: TokenStream, primitives: &HashSet<Ident>) -> TokenStream {
    TokenStream::from({
        let input = parse_macro_input!(input as DeriveInput);
        let generics = &input.generics;
        let data = &input.data;
        let struct_name = &input.ident;
        let (_impl_generics, ty_generics, _where_clause) = generics.split_for_impl();
        let core_crate = format_ident!("{}", core_crate_name());

        // Code adapted from https://blog.turbo.fish/proc-macro-error-handling/
        // Thank you!
        let fields = match data {
            Data::Struct(DataStruct {
                fields: Fields::Named(fields),
                ..
            }) => &fields.named,
            _ => panic!("this derive macro works only on structs with named fields"),
        };
        let attr_fields = fields.into_iter().fold(Vec::default(), |mut v, f| {
            let attrs: Vec<_> = f
                .attrs
                .iter()
                .filter(|attr| attr.path.is_ident("control"))
                .collect();
            if !attrs.is_empty() {
                let is_leaf = parse_control_meta(attrs[0]);
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

        // FOO_SIZE = 3; self.foo contains 3 controlled fields
        fn size_const_id(ident: &Ident) -> Ident {
            format_ident!("{}_SIZE", ident.to_string().to_case(Case::UpperSnake))
        }
        // FOO_NAME = "foo"; self.foo is addressable by "foo"
        fn name_const_id(ident: &Ident) -> Ident {
            let ident_upper = ident.to_string().to_case(Case::UpperSnake);
            format_ident!("{}_NAME", ident_upper)
        }
        // FOO_INDEX = 5; address the foo element with index 5 (and maybe higher if node)
        fn index_const_id(ident: &Ident) -> Ident {
            let ident_upper = ident.to_string().to_case(Case::UpperSnake);
            format_ident!("{}_INDEX", ident_upper)
        }
        // FOO_RANGE_END = 7; address foo's flattened elements with indexes 5, 6, and 7.
        fn index_range_end_const_id(ident: &Ident) -> Ident {
            let ident_upper = ident.to_string().to_case(Case::UpperSnake);
            format_ident!("{}_RANGE_END", ident_upper)
        }

        let mut size_const_ids = Vec::default();
        let mut size_const_values = Vec::default();
        attr_fields.iter().for_each(|(ident, ident_type, is_leaf)| {
            let size_const_name = size_const_id(ident);
            size_const_ids.push(size_const_name.clone());

            if primitives.contains(ident_type) || *is_leaf {
                size_const_values.push(quote! { 1 });
            } else {
                size_const_values.push(quote! { #ident_type::STRUCT_SIZE });
            }
        });
        let size_const_body = quote! {
            #( const #size_const_ids: usize = #size_const_values; )*
        };

        let mut index_const_ids = Vec::default();
        let mut index_const_range_end_ids = Vec::default();
        let mut index_const_values = Vec::default();

        // This loop calculates each field's index.
        //
        // Since proc macros don't have access to any other information than the
        // struct TokenStream, we can't incorporate any sub-structure
        // information (such as how big the field is) except by referring to
        // consts. In other words, if Struct contains EmbeddedStruct, we can't
        // ask how big EmbeddedStruct is, but we can refer to
        // EmbeddedStruct::STRUCT_SIZE and let the compiler figure out that
        // value during the build.
        //
        // Thus, a field's index will always be either (1) zero if it's the
        // first, or (2) the index of the prior field + the size of the prior
        // field. So we need to keep track of the prior field name, which
        // enables us to build up the current value from the prior one.
        let mut prior_ident: Option<&Ident> = None;
        attr_fields.iter().for_each(|(ident, _, _is_leaf)| {
            index_const_ids.push(index_const_id(ident));
            index_const_range_end_ids.push(index_range_end_const_id(ident));
            if let Some(prior) = prior_ident {
                let prior_index_const_name = index_const_id(prior);
                let prior_size_const_name = size_const_id(prior);
                index_const_values
                    .push(quote! { Self::#prior_index_const_name + Self::#prior_size_const_name });
            } else {
                index_const_values.push(quote! { 0 });
            }
            prior_ident = Some(ident);
        });
        let mut name_const_ids = Vec::default();
        let mut name_const_values = Vec::default();
        attr_fields.iter().for_each(|(ident, _, _is_leaf)| {
            let name_const = name_const_id(ident);
            name_const_ids.push(name_const.clone());
            name_const_values.push(ident.to_string().to_case(Case::Kebab));
        });

        let main_const_body = quote! {
            #[allow(missing_docs)]
            #( pub const #index_const_ids: usize = #index_const_values; )*
            #[allow(missing_docs)]
            #( pub const #name_const_ids: &str = #name_const_values; )*
        };
        let range_const_body = quote! {
            #[allow(missing_docs)]
            #( pub const #index_const_range_end_ids: usize = #index_const_values + #size_const_values - 1; )*
        };
        let struct_size_const_body = if size_const_ids.is_empty() {
            quote! {
                #[allow(missing_docs)]
                pub const STRUCT_SIZE: usize = 0;
            }
        } else {
            quote! {
                #[allow(missing_docs)]
                pub const STRUCT_SIZE: usize = 0 + #( Self::#size_const_ids )+* ;
            }
        };

        let mut id_bodies = Vec::default();
        let mut setter_bodies = Vec::default();
        attr_fields.iter().for_each(|(ident, ident_type, is_leaf)| {
            let id = ident.to_string().to_case(Case::Kebab);
            if primitives.contains(ident_type) || *is_leaf {
                let name_const = format_ident!("set_{}", ident);
                id_bodies.push(quote! {Some(#id.to_string())});
                setter_bodies.push(quote! {self.#name_const(value.into());});
            } else {
                let field_index_name = index_const_id(ident);
                let name_const = name_const_id(ident);
                id_bodies.push(quote! { Some(format!("{}-{}", Self::#name_const, self.#ident.control_name_for_index(#core_crate::control::ControlIndex(index.0 - Self::#field_index_name)).unwrap()))});
                setter_bodies
                    .push(quote! {self.#ident.control_set_param_by_index(#core_crate::control::ControlIndex(index.0 - Self::#field_index_name), value);});
            }
        });
        let control_name_for_index_body = quote! {
            fn control_name_for_index(&self, index: #core_crate::control::ControlIndex) -> Option<String> {
                match index.0 {
                    #( Self::#index_const_ids..=Self::#index_const_range_end_ids => {#id_bodies}, )*
                    _ => {None},
                }
            }
        };
        let control_set_param_by_index_bodies = quote! {
            fn control_set_param_by_index(&mut self, index: #core_crate::control::ControlIndex, value: #core_crate::control::ControlValue) {
                match index.0 {
                    #( Self::#index_const_ids..=Self::#index_const_range_end_ids => {#setter_bodies}, )*
                    _ => {},
                }
            }
        };

        // These need to be separate vecs because we divide the fields into
        // groups of maybe different sizes, which is a repetitions no-no.
        let mut leaf_names = Vec::default();
        let mut leaf_indexes = Vec::default();
        let mut node_names = Vec::default();
        let mut node_indexes = Vec::default();
        let mut node_fields = Vec::default();
        let mut node_field_lens = Vec::default();
        attr_fields.iter().for_each(|(ident, ident_type, is_leaf)| {
            let const_name = name_const_id(ident);
            let field_index_name = index_const_id(ident);
            if primitives.contains(ident_type) || *is_leaf {
                leaf_names.push(quote! { Self::#const_name });
                leaf_indexes.push(quote! { Self::#field_index_name });
            } else {
                node_names.push(quote! { Self::#const_name });
                node_indexes.push(quote! { Self::#field_index_name });
                node_fields.push(ident);
                // Includes the dash at the end that separates the field parts in the ID
                let node_field_len = ident.to_string().len() + 1;
                node_field_lens.push(quote! {#node_field_len});
            }
        });
        let control_index_for_name_body = quote! {
            fn control_index_for_name(&self, name: &str) -> Option<#core_crate::control::ControlIndex> {
                match name {
                    #( #leaf_names => Some(#core_crate::control::ControlIndex(#leaf_indexes)), )*
                    _ => {
                        #(
                            if name.starts_with(#node_names) {
                                if let Some(r) = self.#node_fields.control_index_for_name(&name[#node_field_lens..]) {
                                    return Some(#core_crate::control::ControlIndex(r.0 + #node_indexes))
                                }
                            }
                        )*
                        None
                    },
                }
            }
        };

        let quote = quote! {
            #[automatically_derived]
            impl #generics #struct_name #ty_generics {
                #size_const_body
                #main_const_body
                #range_const_body
                #struct_size_const_body
            }
            #[automatically_derived]
            impl #generics #core_crate::traits::Controllable for #struct_name #ty_generics {
                fn control_index_count(&self) -> usize { Self::STRUCT_SIZE }
                fn control_set_param_by_name(&mut self, name: &str, value: #core_crate::control::ControlValue) {
                    if let Some(index) = self.control_index_for_name(name) {
                        self.control_set_param_by_index(index, value);
                    } else {
                        eprintln!("Warning: couldn't set param named '{}'", name);
                    }
                }
                #control_name_for_index_body
                #control_index_for_name_body
                #control_set_param_by_index_bodies
            }
        };
        quote
    })
}

fn parse_control_meta(attr: &Attribute) -> bool {
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
