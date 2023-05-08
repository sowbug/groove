// Copyright (c) 2023 Mike Tsao. All rights reserved.

use quote::{format_ident, quote};
use syn::{Data, DataEnum, Ident, Meta, NestedMeta};

struct ViewableThing {
    base_name: Ident,
    ty: syn::Type,
    is_viewable: bool,
}

fn build_lists<'a>(
    things: impl Iterator<Item = &'a ViewableThing>,
) -> (Vec<Ident>, Vec<syn::Type>, Vec<Ident>, Vec<Ident>) {
    let mut structs = Vec::default();
    let mut types = Vec::default();
    let mut params = Vec::default();
    let mut messages = Vec::default();
    for thing in things {
        params.push(format_ident!("{}Nano", thing.base_name.to_string()));
        messages.push(format_ident!("{}Message", thing.base_name.to_string()));
        types.push(thing.ty.clone());
        structs.push(thing.base_name.clone());
    }
    (structs, types, params, messages)
}

pub(crate) fn parse_and_generate_views(data: &Data) -> proc_macro2::TokenStream {
    let things = match data {
        Data::Enum(DataEnum { variants, .. }) => {
            let mut v = Vec::default();
            for variant in variants.iter() {
                let mut is_viewable = true;
                for attr in &variant.attrs {
                    if let Ok(meta) = attr.parse_meta() {
                        if let Meta::List(list) = meta {
                            if list.path.is_ident("views") {
                                for i in list.nested.iter() {
                                    if let NestedMeta::Meta(m) = i {
                                        if m.path().is_ident("not_viewable") {
                                            is_viewable = false;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                for field in variant.fields.iter() {
                    v.push(ViewableThing {
                        base_name: variant.ident.clone(),
                        ty: field.ty.clone(),
                        is_viewable,
                    });
                }
            }
            v
        }
        _ => panic!("this derive macro works only on enums"),
    };

    let (structs, _, params, _) = build_lists(things.iter());
    let views_enum = quote! {
        #[derive(Debug)]
        pub enum Views {
            #( #structs(Box<#params>) ),*
        }
    };

    let (structs, _, _, messages) = build_lists(things.iter().filter(|thing| thing.is_viewable));
    let viewable_dispatchers = quote! {
        impl View {
            fn entity_view<'a>(&self, uid: usize, entity: &'a Entity) -> Element<'a, ViewMessage> {
                match entity {
                #(
                    Entity::#structs(e) => {
                        e.view().map(move |message| {
                            ViewMessage::OtherEntityMessage(uid, OtherEntityMessage::#structs(message))
                        })
                    }
                ),*
                }
            }

            fn entity_create(
                &mut self,
                uid: usize,
                message: OtherEntityMessage,
            )  {
                match message {
                #(
                    OtherEntityMessage::#structs(#messages::#structs(params)) => {
                      //  self.add_entity(uid, Entity::#structs(Box::new(params)));
                    }
                ),*
                    _=> {
                        panic!("Someone called entity_create with a param-specific message, rather than the full struct message.");
                     }
        }
            }
        }
    };

    quote! {
        #[automatically_derived]
        #views_enum
        #[automatically_derived]
        #viewable_dispatchers
    }
}
