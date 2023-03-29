// Copyright (c) 2023 Mike Tsao. All rights reserved.

//! This crate provides macros that make Entity development easier.

use control::impl_control_derive;
use everything::parse_and_generate_everything;
use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};
use synchronization::impl_synchronization_derive;
use uid::impl_uid_derive;
use views::parse_and_generate_views;

mod control;
mod everything;
mod synchronization;
mod uid;
mod views;

/// The [Uid] macro derives the boilerplate necessary for the HasUid trait. If a
/// device needs to interoperate with Orchestrator, then it needs to have a
/// unique ID. Deriving with this macro makes that happen.
#[proc_macro_derive(Uid)]
pub fn uid_derive(input: TokenStream) -> TokenStream {
    impl_uid_derive(input)
}

/// The [Control] macro derives the infrastructure that makes an entity
/// controllable (automatable). By annotating each controllable field with
/// `#[controllable]`, the entity exposes a public API that Orchestrator uses to
/// manipulate those fields.
#[proc_macro_derive(Control, attributes(controllable))]
pub fn control_derive(input: TokenStream) -> TokenStream {
    impl_control_derive(input)
}

/// The [Synchronization] macro derives the infrastructure that helps sync
/// system data. If you have a struct Foo, then this macro will (eventually)
/// help generate a NanoFoo struct, along with methods and messages that make it
/// easy to propagate changes between the two.
///
/// [Control] and [Synchronization] turned out to have similar solutions, though
/// the problems are different, so I'm in the process of merging them.
#[proc_macro_derive(Synchronization, attributes(sync))]
pub fn synchronization_derive(input: TokenStream) -> TokenStream {
    impl_synchronization_derive(input)
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
