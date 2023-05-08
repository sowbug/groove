// Copyright (c) 2023 Mike Tsao. All rights reserved.

//! This crate provides macros that make Entity development easier.

use control::impl_control_derive;
use everything::parse_and_generate_everything;
use nano::impl_nano_derive;
use prefs::impl_prefs_derive;
use proc_macro::TokenStream;
use proc_macro_crate::crate_name;
use quote::{format_ident, quote};
use std::collections::HashSet;
use syn::{parse_macro_input, DeriveInput, Ident};
use uid::impl_uid_derive;
use views::parse_and_generate_views;

mod control;
mod everything;
mod nano;
mod prefs;
mod uid;
mod views;

/// The [Uid] macro derives the boilerplate necessary for the HasUid trait. If a
/// device needs to interoperate with Orchestrator, then it needs to have a
/// unique ID. Deriving with this macro makes that happen.
#[proc_macro_derive(Uid)]
pub fn uid_derive(input: TokenStream) -> TokenStream {
    impl_uid_derive(input)
}

/// The [Nano] macro helps sync system data. If you have a struct Foo, then this
/// macro generates a FooNano struct, along with methods and messages that make
/// it easy to propagate changes between the two.
#[proc_macro_derive(Nano, attributes(nano))]
pub fn nano_derive(input: TokenStream) -> TokenStream {
    impl_nano_derive(input)
}

/// The [Prefs] macro generates helper structs that are useful for handing
/// around bundles of arguments. If you have a struct Foo, then this macro
/// generates a FooPrefs struct containing all the fields annotated #[prefs]. It
/// automatically converts fields whose types are #[derive(Prefs)] to Prefs
/// structs as well. So a `#[derive(Prefs)] struct Foo` with `#[prefs] bar: Bar`
/// will generate `struct FooPrefs` with `bar: BarPrefs`.
#[proc_macro_derive(Prefs, attributes(prefs))]
pub fn prefs_derive(input: TokenStream) -> TokenStream {
    impl_prefs_derive(input, &make_primitives())
}

/// The [Everything] macro derives the code that ties all Entities together.
#[proc_macro_derive(Everything, attributes(everything))]
pub fn derive_everything(input: TokenStream) -> TokenStream {
    TokenStream::from(parse_and_generate_everything(
        &(parse_macro_input!(input as DeriveInput)).data,
    ))
}

/// The [Views] macro derives code that presents viewable entities as a single
/// system.
#[proc_macro_derive(Views, attributes(views))]
pub fn derive_views(input: TokenStream) -> TokenStream {
    TokenStream::from(parse_and_generate_views(
        &(parse_macro_input!(input as DeriveInput)).data,
    ))
}

/// field types that don't recurse further for #[derive(Control)] purposes
fn make_primitives() -> HashSet<Ident> {
    vec![
        "BipolarNormal",
        "FrequencyHz",
        "Normal",
        "ParameterType",
        "Ratio",
        "String",
        "Waveform",
        "bool",
        "char",
        "f32",
        "f64",
        "i128",
        "i16",
        "i32",
        "i64",
        "i8",
        "u128",
        "u16",
        "u32",
        "u64",
        "u8",
        "usize",
    ]
    .into_iter()
    .fold(HashSet::default(), |mut hs, e| {
        hs.insert(format_ident!("{}", e));
        hs
    })
}

/// The [Control] macro derives the code that allows automation (one entity's output driving another entity's control).
#[proc_macro_derive(Control, attributes(control))]
pub fn derive_control(input: TokenStream) -> TokenStream {
    impl_control_derive(input, &make_primitives())
}

// Some of the code generated in these macros uses the groove-core crate, but
// groove-core also uses this proc-macro lib. So we need to correct the
// reference to groove-core to sometimes be just `crate`.
fn core_crate_name() -> String {
    const CORE_CRATE_NAME: &str = "groove-core";
    const CORE_CRATE_NAME_FOR_USE: &str = "groove_core";
    if let Ok(found_crate) = crate_name(CORE_CRATE_NAME) {
        match found_crate {
            proc_macro_crate::FoundCrate::Itself => {
                // We aren't importing the crate by name, so we must be it.
                quote!(crate).to_string()
            }
            proc_macro_crate::FoundCrate::Name(_) => {
                // We're importing the crate by name, which means we aren't the core crate.
                let ident = format_ident!("{}", CORE_CRATE_NAME_FOR_USE);
                quote!(#ident).to_string()
            }
        }
    } else {
        panic!("forgot to import {}", CORE_CRATE_NAME);
    }
}
