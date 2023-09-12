// Copyright (c) 2023 Mike Tsao. All rights reserved.

use crate::core_crate_name;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, DeriveInput};

pub(crate) enum EntityType {
    Controller,
    Effect,
    Instrument,
    ControllerEffect,
    ControllerInstrument,
}

// TODO: see
// https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=03943d1dfbf41bd63878bfccb1c64670
// for an intriguing bit of code. Came from
// https://users.rust-lang.org/t/is-implementing-a-derive-macro-for-converting-nested-structs-to-flat-structs-possible/65839/3

pub(crate) fn parse_and_generate_entity(input: TokenStream, ty: EntityType) -> TokenStream {
    TokenStream::from({
        let input = parse_macro_input!(input as DeriveInput);
        let generics = &input.generics;
        let struct_name = &input.ident;
        let (_impl_generics, ty_generics, _where_clause) = generics.split_for_impl();
        let core_crate = format_ident!("{}", core_crate_name());

        let top_level_trait_names = match ty {
            EntityType::Controller => vec![quote! {#core_crate::traits::IsController}],
            EntityType::Effect => vec![quote! {#core_crate::traits::IsEffect}],
            EntityType::Instrument => vec![quote! {#core_crate::traits::IsInstrument}],
            EntityType::ControllerEffect => vec![
                quote! {#core_crate::traits::IsController},
                quote! {#core_crate::traits::IsEffect},
            ],
            EntityType::ControllerInstrument => vec![
                quote! {#core_crate::traits::IsController},
                quote! {#core_crate::traits::IsInstrument},
            ],
        };
        let common_items = quote! {};
        let type_specific_items = match ty {
            EntityType::Controller => quote! {
                fn as_controller(&self) -> Option<&dyn #core_crate::traits::IsController> {
                    Some(self)
                }
                fn as_controller_mut(&mut self) -> Option<&mut dyn #core_crate::traits::IsController> {
                    Some(self)
                }
            },
            EntityType::Effect => quote! {
                fn as_effect(&self) -> Option<&dyn #core_crate::traits::IsEffect> {
                    Some(self)
                }
                fn as_effect_mut(&mut self) -> Option<&mut dyn #core_crate::traits::IsEffect> {
                    Some(self)
                }
            },
            EntityType::Instrument => quote! {
                fn as_instrument(&self) -> Option<&dyn #core_crate::traits::IsInstrument> {
                    Some(self)
                }
                fn as_instrument_mut(&mut self) -> Option<&mut dyn #core_crate::traits::IsInstrument> {
                    Some(self)
                }
            },
            EntityType::ControllerEffect => quote! {
                fn as_controller(&self) -> Option<&dyn #core_crate::traits::IsController> {
                    Some(self)
                }
                fn as_controller_mut(&mut self) -> Option<&mut dyn #core_crate::traits::IsController> {
                    Some(self)
                }
                fn as_effect(&self) -> Option<&dyn #core_crate::traits::IsEffect> {
                    Some(self)
                }
                fn as_effect_mut(&mut self) -> Option<&mut dyn #core_crate::traits::IsEffect> {
                    Some(self)
                }
            },
            EntityType::ControllerInstrument => quote! {
                fn as_controller(&self) -> Option<&dyn #core_crate::traits::IsController> {
                    Some(self)
                }
                fn as_controller_mut(&mut self) -> Option<&mut dyn #core_crate::traits::IsController> {
                    Some(self)
                }
                fn as_instrument(&self) -> Option<&dyn #core_crate::traits::IsInstrument> {
                    Some(self)
                }
                fn as_instrument_mut(&mut self) -> Option<&mut dyn #core_crate::traits::IsInstrument> {
                    Some(self)
                }
            },
        };
        let handles_midi_items = match ty {
            EntityType::Controller
            | EntityType::Instrument
            | EntityType::ControllerEffect
            | EntityType::ControllerInstrument => quote! {
                fn as_handles_midi(&self) -> Option<&dyn #core_crate::traits::HandlesMidi> {
                    Some(self)
                }
                fn as_handles_midi_mut(&mut self) -> Option<&mut dyn #core_crate::traits::HandlesMidi> {
                    Some(self)
                }
            },
            EntityType::Effect => quote! {},
        };
        let controllable_items = match ty {
            EntityType::Controller => quote! {},
            EntityType::Effect
            | EntityType::Instrument
            | EntityType::ControllerEffect
            | EntityType::ControllerInstrument => quote! {
                fn as_controllable(&self) -> Option<&dyn #core_crate::traits::Controllable> {
                    Some(self)
                }
                fn as_controllable_mut(&mut self) -> Option<&mut dyn #core_crate::traits::Controllable> {
                    Some(self)
                }
            },
        };

        let quote = quote! {
            #[automatically_derived]
            #( impl #generics #top_level_trait_names for #struct_name #ty_generics {} )*

            #[automatically_derived]
            #[typetag::serde]
            impl #generics #core_crate::traits::Entity for #struct_name #ty_generics {
                #common_items
                #type_specific_items
                #handles_midi_items
                #controllable_items
            }
        };
        quote
    })
}
