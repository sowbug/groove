use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DataStruct, DeriveInput, Fields, Generics};

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

    // Code adapted from https://blog.turbo.fish/proc-macro-error-handling/
    // Thank you!
    let fields = match data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => &fields.named,
        _ => panic!("this derive macro only works on structs with named fields"),
    };
    let controllables: Vec<_> = fields
        .into_iter()
        .filter(|f| {
            let attrs: Vec<_> = f
                .attrs
                .iter()
                .filter(|attr| attr.path.is_ident("controllable"))
                .collect();
            !attrs.is_empty()
        })
        .map(|f| f.ident.as_ref().unwrap().to_string())
        .collect();
    //    eprintln!("for {}, {:#?}", &struct_name, &controllables);

    for name in controllables {
        enum_variant_names.push(format_ident!("{}", name.to_case(Case::Pascal)));

        // TODO: come up with a way to make this kind of method more general. It
        // would be nice if there were a "convert F32ControlValue to local
        // value" function that anyone (e.g., UI) could use. But then we'd also
        // need to know the name of the local setter method. While that's not
        // necessarily a bad thing, we're already seeing cases where the
        // controlled value doesn't actually correspond to a struct field (e.g.,
        // BiQuadFilter's param2), and I'm not sure I want to normalize that.
        setter_names.push(format_ident!("set_control_{}", name));
    }

    let enum_block = quote! {
        #[derive(Display, Debug, EnumString, FromRepr)]
        #[strum(serialize_all = "kebab_case")]
        pub(crate) enum #enum_name {
            #( #enum_variant_names ),*
        }
    };
    let controllable_block = quote! {
        impl #generics Controllable for #struct_name #ty_generics {
            fn control_index_for_name(&self, name: &str) -> usize {
                if let Ok(param) = #enum_name::from_str(name) {
                    param as usize
                } else {
                    eprintln!("Unrecognized control param name: {}", name);
                    usize::MAX
                }
            }
            fn set_by_control_index(&mut self, index: usize, value: F32ControlValue) {
                if let Some(param) = #enum_name::from_repr(index) {
                    match param {
                        #( #enum_name::#enum_variant_names => {self.#setter_names(value); } ),*
                    }
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
