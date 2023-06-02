// Copyright (c) 2023 Mike Tsao. All rights reserved.

use crate::core_crate_name;
use quote::{format_ident, quote};
use syn::{Data, DataEnum, Ident, Meta, NestedMeta};

struct OneThing {
    base_name: Ident,
    ty: syn::Type,
    is_controller: bool,
    is_effect: bool,
    is_instrument: bool,
    is_controllable: bool,
    handles_midi: bool,
}

fn build_lists<'a>(things: impl Iterator<Item = &'a OneThing>) -> (Vec<Ident>, Vec<syn::Type>) {
    let mut structs = Vec::default();
    let mut types = Vec::default();
    for thing in things {
        types.push(thing.ty.clone());
        structs.push(thing.base_name.clone());
    }
    (structs, types)
}

pub(crate) fn parse_and_generate_everything(data: &Data) -> proc_macro2::TokenStream {
    let things = match data {
        Data::Enum(DataEnum { variants, .. }) => {
            let mut v = Vec::default();
            for variant in variants.iter() {
                let mut is_controller = false;
                let mut is_effect = false;
                let mut is_instrument = false;
                let mut is_controllable = false;
                let mut handles_midi = false;
                for attr in &variant.attrs {
                    if let Ok(meta) = attr.parse_meta() {
                        if let Meta::List(list) = meta {
                            if list.path.is_ident("everything") {
                                for i in list.nested.iter() {
                                    if let NestedMeta::Meta(m) = i {
                                        if m.path().is_ident("controller") {
                                            is_controller = true;
                                        }
                                        if m.path().is_ident("effect") {
                                            is_effect = true;
                                        }
                                        if m.path().is_ident("instrument") {
                                            is_instrument = true;
                                        }
                                        if m.path().is_ident("controllable") {
                                            is_controllable = true;
                                        }
                                        if m.path().is_ident("midi") {
                                            handles_midi = true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                for field in variant.fields.iter() {
                    v.push(OneThing {
                        base_name: variant.ident.clone(),
                        ty: field.ty.clone(),
                        is_controller,
                        is_effect,
                        is_instrument,
                        is_controllable,
                        handles_midi,
                    });
                }
            }
            v
        }
        _ => panic!("this derive macro works only on enums"),
    };

    let core_crate = format_ident!("{}", core_crate_name());
    let (structs, types) = build_lists(things.iter());
    let entity_enum = quote! {
        #[derive(Debug)]
        #[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
        pub enum Entity {
            #( #structs(Box<#types>) ),*
        }
    };

    let common_dispatchers = quote! {
        impl Entity {
            pub fn name(&self) -> &str {
                match self {
                    #( Entity::#structs(e) => e.name(), )*
                }
            }
            pub fn as_has_uid(&self) -> &dyn #core_crate::traits::HasUid {
                match self {
                #( Entity::#structs(e) => e.as_ref(), )*
                }
            }
            pub fn as_has_uid_mut(&mut self) -> &mut dyn #core_crate::traits::HasUid {
                match self {
                #( Entity::#structs(e) => e.as_mut(), )*
                }
            }
            pub fn as_configurable_mut(&mut self) -> &mut dyn #core_crate::traits::Configurable {
                match self {
                #( Entity::#structs(e) => e.as_mut(), )*
                }
            }
        }
    };

    let (structs, _) = build_lists(things.iter().filter(|thing| thing.is_controller));
    let controller_dispatchers = quote! {
        impl Entity {
            pub fn is_controller(&self) -> bool {
                match self {
                    #( Entity::#structs(_) => true, )*
                    _ => false,
                }
            }
            pub fn as_is_controller(&self) -> Option<&dyn #core_crate::traits::IsController<Message=MsgType>> {
                match self {
                    #( Entity::#structs(e) => Some(e.as_ref()), )*
                    _ => None,
                }
            }
            pub fn as_is_controller_mut(&mut self) -> Option<&mut dyn #core_crate::traits::IsController<Message=MsgType>> {
                match self {
                    #( Entity::#structs(e) => Some(e.as_mut()), )*
                    _ => None,
                }
            }
        }
    };

    let (structs, _) = build_lists(things.iter().filter(|thing| thing.is_controllable));
    let controllable_dispatchers = quote! {
        impl Entity {
            pub fn is_controllable(&self) -> bool {
                match self {
                    #( Entity::#structs(_) => true, )*
                    _ => false,
                }
            }
            pub fn as_controllable(&self) -> Option<&dyn #core_crate::traits::Controllable> {
                match self {
                    #( Entity::#structs(e) => Some(e.as_ref()), )*
                    _ => None,
                }
            }
            pub fn as_controllable_mut(&mut self) -> Option<&mut dyn #core_crate::traits::Controllable> {
                match self {
                    #( Entity::#structs(e) => Some(e.as_mut()), )*
                    _ => None,
                }
            }
        }
    };

    let (structs, _) = build_lists(things.iter().filter(|thing| thing.is_effect));
    let effect_dispatchers = quote! {
        impl Entity {
            pub fn as_is_effect(&self) -> Option<&dyn #core_crate::traits::IsEffect> {
                match self {
                    #( Entity::#structs(e) => Some(e.as_ref()), )*
                    _ => None,
                }
            }
            pub fn as_is_effect_mut(&mut self) -> Option<&mut dyn #core_crate::traits::IsEffect> {
                match self {
                    #( Entity::#structs(e) => Some(e.as_mut()), )*
                    _ => None,
                }
            }
        }
    };

    let (structs, _) = build_lists(things.iter().filter(|thing| thing.is_instrument));
    let instrument_dispatchers = quote! {
        impl Entity {
            pub fn as_is_instrument(&self) -> Option<&dyn #core_crate::traits::IsInstrument> {
                match self {
                    #( Entity::#structs(e) => Some(e.as_ref()), )*
                    _ => None,
                }
            }
            pub fn as_is_instrument_mut(&mut self) -> Option<&mut dyn #core_crate::traits::IsInstrument> {
                match self {
                    #( Entity::#structs(e) => Some(e.as_mut()), )*
                    _ => None,
                }
            }
        }
    };

    let (structs, _) = build_lists(things.iter().filter(|thing| thing.handles_midi));
    let handles_midi_dispatchers = quote! {
        impl Entity {
            pub fn as_handles_midi(&self) -> Option<&dyn #core_crate::traits::HandlesMidi> {
                match self {
                    #( Entity::#structs(e) => Some(e.as_ref()), )*
                    _ => None,
                }
            }
            pub fn as_handles_midi_mut(&mut self) -> Option<&mut dyn #core_crate::traits::HandlesMidi> {
                match self {
                    #( Entity::#structs(e) => Some(e.as_mut()), )*
                    _ => None,
                }
            }
        }
    };

    quote! {
        #[automatically_derived]
        #entity_enum
        #[automatically_derived]
        #common_dispatchers
        #[automatically_derived]
        #controller_dispatchers
        #[automatically_derived]
        #effect_dispatchers
        #[automatically_derived]
        #instrument_dispatchers
        #[automatically_derived]
        #controllable_dispatchers
        #[automatically_derived]
        #handles_midi_dispatchers
    }
}
