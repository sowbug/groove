// Copyright (c) 2023 Mike Tsao. All rights reserved.

//! This crate provides macros that make Entity development easier.

use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, Attribute, Data, DataEnum, DataStruct, DeriveInput, Fields, Generics, Ident,
    Lit, Meta, NestedMeta,
};

/// The Uid macro derives the boilerplate necessary for the HasUid trait. If a
/// device needs to interoperate with Orchestrator, then it needs to have a
/// unique ID. Deriving with this macro makes that happen.
#[proc_macro_derive(Uid)]
pub fn uid_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let generics = input.generics;
    let (_impl_generics, ty_generics, _where_clause) = generics.split_for_impl();

    let expanded = quote! {
        #[automatically_derived]
        impl #generics groove_core::traits::HasUid for #name #ty_generics {
            fn uid(&self) -> usize {
                self.uid
            }

            fn set_uid(&mut self, uid: usize) {
                self.uid = uid;
            }

            fn name(&self) -> &'static str {
                stringify!(#name)
            }
        }
    };
    TokenStream::from(expanded)
}

/// The Control macro derives the infrastructure that makes an entity
/// controllable (automatable). By annotating each controllable field with
/// `#[controllable]`, the entity exposes a public API that Orchestrator uses to
/// manipulate those fields. Note that adding the `#[controllable]` annotation
/// isn't enough; for each field, you should also add a method called
/// `set_control_FIELDNAME()` that takes a control type, such as
/// [F32ControlValue](groove_core::control::F32ControlValue).
#[proc_macro_derive(Control, attributes(controllable))]
pub fn control_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = input.ident;
    let enum_name = format_ident!("{}ControlParams", struct_name);
    TokenStream::from(parse_control_data(
        &struct_name,
        &input.generics,
        &enum_name,
        &input.data,
    ))
}

fn get_name_values(attr: &Attribute) -> Vec<String> {
    if let Ok(meta) = attr.parse_meta() {
        let meta_list = match meta {
            Meta::List(list) => list,
            _ => {
                return Vec::default();
            }
        };

        let punctuated = match meta_list.nested.len() {
            0 => return Vec::default(),
            _ => &meta_list.nested,
        };

        let values = punctuated.iter().fold(Vec::default(), |mut v, nested| {
            if let NestedMeta::Meta(Meta::NameValue(name_value)) = nested {
                if name_value.path.is_ident("name") {
                    match &name_value.lit {
                        Lit::Str(s) => {
                            v.push(s.value());
                        }
                        _ => {
                            // Err: expected string literal
                        }
                    }
                } else {
                    // Err: unsupported attribute
                }
            } else {
                // Err: unexpected junk when we were looking for name=v
            }
            v
        });
        values
    } else {
        Vec::default()
    }
}

fn parse_control_data(
    struct_name: &Ident,
    generics: &Generics,
    enum_name: &Ident,
    data: &Data,
) -> proc_macro2::TokenStream {
    let (_impl_generics, ty_generics, _where_clause) = generics.split_for_impl();
    let mut enum_variant_names = Vec::default();
    let mut setter_names = Vec::default();

    // Code adapted from https://blog.turbo.fish/proc-macro-error-handling/
    // Thank you!
    let fields = match data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => &fields.named,
        _ => panic!("this derive macro only works on structs with named fields"),
    };
    let controllables: Vec<String> = fields.into_iter().fold(Vec::default(), |mut v, f| {
        let attrs: Vec<_> = f
            .attrs
            .iter()
            .filter(|attr| attr.path.is_ident("controllable"))
            .collect();
        if !attrs.is_empty() {
            let mut values = get_name_values(attrs[0]);
            if values.is_empty() {
                values.push(f.ident.as_ref().unwrap().to_string());
            }
            v.extend(values);
        }
        v
    });

    for name in controllables {
        enum_variant_names.push(format_ident!("{}", name.to_case(Case::Pascal)));

        // TODO: come up with a way to make this kind of method more general. It
        // would be nice if there were a "convert F32ControlValue to local
        // value" function that anyone (e.g., UI) could use. But then we'd also
        // need to know the name of the local setter method. While that's not
        // necessarily a bad thing, we're already seeing cases where the
        // controlled value doesn't actually correspond to a struct field (e.g.,
        // BiQuadFilter's param2), and I'm not sure I want to normalize that.
        setter_names.push(format_ident!("set_control_{}", name.to_case(Case::Snake)));
    }

    let enum_block = quote! {
        #[derive(Display, Debug, EnumCountMacro, EnumIter, EnumString, FromRepr, IntoStaticStr)]
        #[strum(serialize_all = "kebab_case")]
        pub enum #enum_name {
            #( #enum_variant_names ),*
        }
    };
    let controllable_block = quote! {
        impl #generics groove_core::traits::Controllable for #struct_name #ty_generics {
            fn control_index_count(&self) -> usize {
                #enum_name::COUNT
            }
            fn control_index_for_name(&self, name: &str) -> usize {
                if let Ok(param) = #enum_name::from_str(name) {
                    param as usize
                } else {
                    eprintln!("Unrecognized control param name: {}", name);
                    usize::MAX
                }
            }
            fn control_name_for_index(&self, index: usize) -> Option<&'static str> {
                if let Some(param) = #enum_name::from_repr(index) {
                    Some(param.into())
                } else {
                    None
                }
            }
        }
    };
    quote! {
        #[automatically_derived]
        #enum_block
        #[automatically_derived]
        #controllable_block

    }
}

#[proc_macro_derive(Synchronization, attributes(sync))]
pub fn synchronization_derive(input: TokenStream) -> TokenStream {
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

#[proc_macro_derive(Everything, attributes(everything))]
pub fn derive_everything(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    TokenStream::from(parse_and_generate_everything(&input.data))
}

#[derive(Debug)]
struct OneThing {
    base_name: Ident,
    ty: syn::Type,
    is_controller: bool,
    is_effect: bool,
    is_instrument: bool,
    is_controllable: bool,
    handles_midi: bool,
}

fn build_lists<'a>(
    things: impl Iterator<Item = &'a OneThing>,
) -> (Vec<Ident>, Vec<Ident>, Vec<syn::Type>) {
    let mut structs = Vec::default();
    let mut params = Vec::default();
    let mut types = Vec::default();
    for thing in things {
        params.push(format_ident!("{}Params", thing.base_name.to_string()));
        structs.push(thing.base_name.clone());
        types.push(thing.ty.clone());
    }
    (structs, params, types)
}

fn parse_and_generate_everything(data: &Data) -> proc_macro2::TokenStream {
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
                                            if m.path().is_ident("controller") {
                                                is_controller = true;
                                            }
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
                for (field_index, field) in variant.fields.iter().enumerate() {
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

    let (structs, params, types) = build_lists(things.iter());

    let entity_enum = quote! {
        #[derive(Debug)]
        pub enum Entity {
            #( #structs(Box<#types>) ),*
        }

        #[derive(Debug)]
        pub enum EntityParams {
            #( #structs(Box<#params>) ),*
        }
    };

    let common_upcasters = quote! {
        impl Entity {
            pub fn as_has_uid(&self) -> &dyn HasUid {
                match self {
                #( Entity::#structs(e) => e.as_ref(), )*
                }
            }
            pub fn as_has_uid_mut(&mut self) -> &mut dyn HasUid {
                match self {
                #( Entity::#structs(e) => e.as_mut(), )*
                }
            }
        }
    };

    let (structs, params, types) = build_lists(things.iter().filter(|thing| thing.is_controller));
    let controller_upcasters = quote! {
        impl Entity {
            pub fn is_controller(&self) -> bool {
                match self {
                    #( Entity::#structs(_) => true, )*
                    _ => false,
                }
            }
            pub fn as_is_controller(&self) -> Option<&dyn groove_core::traits::IsController<Message=Moosage>> {
                match self {
                    #( Entity::#structs(e) => Some(e.as_ref()), )*
                    _ => None,
                }
            }
            pub fn as_is_controller_mut(&mut self) -> Option<&mut dyn groove_core::traits::IsController<Message=Moosage>> {
                match self {
                    #( Entity::#structs(e) => Some(e.as_mut()), )*
                    _ => None,
                }
            }
        }
        impl EntityParams {
            pub fn is_controller(&self) -> bool {
                match self {
                    #( EntityParams::#structs(_) => true, )*
                    _ => false,
                }
            }
        }
    };

    let (structs, params, types) = build_lists(things.iter().filter(|thing| thing.is_controllable));
    let controllable_upcasters = quote! {
        impl Entity {
            pub fn is_controllable(&self) -> bool {
                match self {
                    #( Entity::#structs(_) => true, )*
                    _ => false,
                }
            }
            pub fn as_controllable(&self) -> Option<&dyn groove_core::traits::Controllable> {
                match self {
                    #( Entity::#structs(e) => Some(e.as_ref()), )*
                    _ => None,
                }
            }
            pub fn as_controllable_mut(&mut self) -> Option<&mut dyn groove_core::traits::Controllable> {
                match self {
                    #( Entity::#structs(e) => Some(e.as_mut()), )*
                    _ => None,
                }
            }
        }
        impl EntityParams {
            pub fn is_controllable(&self) -> bool {
                match self {
                    #( EntityParams::#structs(_) => true, )*
                    _ => false,
                }
            }
            pub fn as_controllable(&self) -> Option<&dyn groove_core::traits::Controllable> {
                match self {
                    #( EntityParams::#structs(e) => Some(e.as_ref()), )*
                    _ => None,
                }
            }
            pub fn as_controllable_mut(&mut self) -> Option<&mut dyn groove_core::traits::Controllable> {
                match self {
                    #( EntityParams::#structs(e) => Some(e.as_mut()), )*
                    _ => None,
                }
            }
        }
    };

    let (structs, params, types) = build_lists(things.iter().filter(|thing| thing.is_effect));
    let effect_upcasters = quote! {
        impl Entity {
            pub fn as_is_effect(&self) -> Option<&dyn groove_core::traits::IsEffect> {
                match self {
                    #( Entity::#structs(e) => Some(e.as_ref()), )*
                    _ => None,
                }
            }
            pub fn as_is_effect_mut(&mut self) -> Option<&mut dyn groove_core::traits::IsEffect> {
                match self {
                    #( Entity::#structs(e) => Some(e.as_mut()), )*
                    _ => None,
                }
            }
        }
    };

    let (structs, params, types) = build_lists(things.iter().filter(|thing| thing.is_instrument));
    let instrument_upcasters = quote! {
        impl Entity {
            pub fn as_is_instrument(&self) -> Option<&dyn groove_core::traits::IsInstrument> {
                match self {
                    #( Entity::#structs(e) => Some(e.as_ref()), )*
                    _ => None,
                }
            }
            pub fn as_is_instrument_mut(&mut self) -> Option<&mut dyn groove_core::traits::IsInstrument> {
                match self {
                    #( Entity::#structs(e) => Some(e.as_mut()), )*
                    _ => None,
                }
            }
        }
    };

    let (structs, params, types) = build_lists(things.iter().filter(|thing| thing.handles_midi));
    let handles_midi_upcasters = quote! {
        impl Entity {
            pub fn as_handles_midi(&self) -> Option<&dyn groove_core::traits::HandlesMidi> {
                match self {
                    #( Entity::#structs(e) => Some(e.as_ref()), )*
                    _ => None,
                }
            }
            pub fn as_handles_midi_mut(&mut self) -> Option<&mut dyn groove_core::traits::HandlesMidi> {
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
        #common_upcasters
        #[automatically_derived]
        #controller_upcasters
        #[automatically_derived]
        #effect_upcasters
        #[automatically_derived]
        #instrument_upcasters
        #[automatically_derived]
        #controllable_upcasters
        #[automatically_derived]
        #handles_midi_upcasters

        enum Foo {
            #( #structs(#types) ),*
        }
    }
}
