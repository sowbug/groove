use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Fields, Generics};

#[proc_macro_derive(Uid)]
pub fn uid_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let generics = input.generics;
    let (_impl_generics, ty_generics, _where_clause) = generics.split_for_impl();

    let expanded = quote! {
        #[automatically_derived]
        impl #generics HasUid for #name #ty_generics {
            fn uid(&self) -> usize {
                self.uid
            }

            fn set_uid(&mut self, uid: usize) {
                self.uid = uid;
            }
        }
    };
    TokenStream::from(expanded)
}

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

fn parse_control_data(
    struct_name: &Ident,
    generics: &Generics,
    enum_name: &Ident,
    data: &Data,
) -> proc_macro2::TokenStream {
    let (_impl_generics, ty_generics, _where_clause) = generics.split_for_impl();
    let mut enum_variant_names = Vec::default();
    let mut setter_names = Vec::default();
    match *data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                for f in fields.named.iter() {
                    if let Some(ident) = &f.ident {
                        if f.attrs.is_empty() {
                            continue;
                        }
                        enum_variant_names
                            .push(format_ident!("{}", ident.to_string().to_case(Case::Pascal)));
                        // TODO: this should be an Into-style method for anyone to use to convert from
                        // normalized control values to the specific variable's needs.
                        setter_names.push(format_ident!("set_control_{}", ident.to_string()));
                    }
                }
            }
            Fields::Unnamed(ref _fields) => unimplemented!(),
            Fields::Unit => unimplemented!(),
        },
        Data::Enum(_) | Data::Union(_) => unimplemented!(),
    }
    let enum_block = quote! {
        #[automatically_derived]
        #[derive(Display, Debug, EnumString, FromRepr)]
        #[strum(serialize_all = "kebab_case")]
        pub(crate) enum #enum_name {
            #( #enum_variant_names ),*
        }
    };
    let controllable_block = quote! {
        #[automatically_derived]
        impl #generics Controllable for #struct_name #ty_generics {
            fn control_index_for_name(&self, name: &str) -> usize {
                if let Ok(param) = #enum_name::from_str(name) {
                    param as usize
                } else {
                    usize::MAX
                }
            }
            fn set_by_control_index(&mut self, index: usize, value: F32ControlValue) {
                if let Some(param) = #enum_name::from_repr(index) {
                    match param {
                        #( #enum_name::#enum_variant_names => {self.#setter_names(value); } ),*
                    }
                } else {
                    todo!()
                }
            }
        }
    };
    quote! {
        #enum_block
        #controllable_block
    }
}
