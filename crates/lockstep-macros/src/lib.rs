// SPDX-License-Identifier: Apache-2.0
//! `#[derive(Message)]` for lockstep-core. Use it through `lockstep_core::Message`.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as Tokens;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{parse_macro_input, Data, DeriveInput, Fields, LitInt, Type};

/// Implements `lockstep_core::Message` with the version from `#[message(version = N)]`, and
/// `lockstep_core::Indexable`: the kind is an enum's variant index (0 for a struct), and the
/// handles are every field of type `Handle`, `Option<Handle>` or `Vec<Handle>`, in field order.
#[proc_macro_derive(Message, attributes(message))]
pub fn derive_message(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand(&input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

fn expand(input: &DeriveInput) -> syn::Result<Tokens> {
    let version = version(input)?;
    let name = &input.ident;
    // A generic message is a message only when its shape can be saved: the same bounds the
    // trait asks of every message, stated on `Self`.
    let mut generics = input.generics.clone();
    if !generics.params.is_empty() {
        generics
            .make_where_clause()
            .predicates
            .push(syn::parse_quote! {
                Self: ::lockstep_core::__private::Serialize
                    + ::lockstep_core::__private::DeserializeOwned
                    + ::std::clone::Clone
                    + ::std::cmp::PartialEq
                    + ::std::fmt::Debug
            });
    }
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let (kind, handles) = match &input.data {
        Data::Struct(data) => {
            let (pattern, pushes) = destructure(&data.fields);
            (
                quote! { 0 },
                quote! {
                    let Self #pattern = self;
                    #pushes
                },
            )
        }
        Data::Enum(data) => {
            if data.variants.len() > u16::MAX as usize + 1 {
                return Err(syn::Error::new(
                    input.ident.span(),
                    "a message enum may have at most 65,536 variants",
                ));
            }
            let kinds = data.variants.iter().enumerate().map(|(index, variant)| {
                let variant_name = &variant.ident;
                let index = index as u16;
                let ignored = match &variant.fields {
                    Fields::Named(_) => quote! { { .. } },
                    Fields::Unnamed(_) => quote! { (..) },
                    Fields::Unit => quote! {},
                };
                quote! { Self::#variant_name #ignored => #index, }
            });
            let handle_arms = data.variants.iter().map(|variant| {
                let variant_name = &variant.ident;
                let (pattern, pushes) = destructure(&variant.fields);
                quote! { Self::#variant_name #pattern => { #pushes } }
            });
            if data.variants.is_empty() {
                // No value of an empty enum exists; dereferencing proves it to the compiler.
                (quote! { match *self {} }, quote! { match *self {} })
            } else {
                (
                    quote! { match self { #(#kinds)* } },
                    quote! {
                        match self { #(#handle_arms)* }
                    },
                )
            }
        }
        Data::Union(_) => {
            return Err(syn::Error::new(
                input.ident.span(),
                "a message cannot be a union",
            ))
        }
    };
    Ok(quote! {
        impl #impl_generics ::lockstep_core::Message for #name #type_generics #where_clause {
            const VERSION: u32 = #version;
        }

        impl #impl_generics ::lockstep_core::Indexable for #name #type_generics #where_clause {
            fn kind(&self) -> u16 {
                #kind
            }

            #[allow(unused_variables)]
            fn handles(&self, out: &mut ::std::vec::Vec<::lockstep_core::Handle>) {
                #handles
            }
        }
    })
}

/// Reads `#[message(version = N)]`, which every message must carry exactly once.
fn version(input: &DeriveInput) -> syn::Result<LitInt> {
    let mut found: Option<LitInt> = None;
    for attribute in input
        .attrs
        .iter()
        .filter(|attribute| attribute.path().is_ident("message"))
    {
        attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident("version") {
                let value: LitInt = meta.value()?.parse()?;
                value.base10_parse::<u32>()?;
                if found.replace(value).is_some() {
                    return Err(meta.error("the version is given twice"));
                }
                Ok(())
            } else {
                Err(meta.error("expected `version = N`"))
            }
        })?;
    }
    found.ok_or_else(|| {
        syn::Error::new(
            input.ident.span(),
            "a message needs `#[message(version = N)]`: a shipped shape is never edited, so every change is a new version",
        )
    })
}

/// A pattern binding every field, and the code that pushes the handles among them.
fn destructure(fields: &Fields) -> (Tokens, Tokens) {
    let bindings: Vec<_> = (0..fields.len())
        .map(|index| quote::format_ident!("field_{}", index))
        .collect();
    let pushes = fields.iter().zip(&bindings).map(|(field, binding)| {
        let span = field.ty.span();
        match handle_shape(&field.ty) {
            Some(Shape::One) => quote_spanned! { span=> out.push(*#binding); },
            Some(Shape::Optional) => {
                quote_spanned! { span=> if let ::std::option::Option::Some(handle) = #binding { out.push(*handle); } }
            }
            Some(Shape::Many) => quote_spanned! { span=> out.extend(#binding.iter().copied()); },
            None => quote! {},
        }
    });
    let pattern = match fields {
        Fields::Named(_) => {
            let names = fields
                .iter()
                .map(|field| field.ident.as_ref().expect("named"));
            quote! { { #(#names: #bindings),* } }
        }
        Fields::Unnamed(_) => quote! { ( #(#bindings),* ) },
        Fields::Unit => quote! {},
    };
    (pattern, quote! { #(#pushes)* })
}

enum Shape {
    One,
    Optional,
    Many,
}

fn last_segment(ty: &Type) -> Option<&syn::PathSegment> {
    match ty {
        Type::Path(path) if path.qself.is_none() => path.path.segments.last(),
        _ => None,
    }
}

fn is_handle(ty: &Type) -> bool {
    last_segment(ty).is_some_and(|segment| segment.ident == "Handle" && segment.arguments.is_none())
}

fn handle_shape(ty: &Type) -> Option<Shape> {
    if is_handle(ty) {
        return Some(Shape::One);
    }
    let segment = last_segment(ty)?;
    let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };
    let mut types = arguments.args.iter().filter_map(|argument| match argument {
        syn::GenericArgument::Type(ty) => Some(ty),
        _ => None,
    });
    let inner = types.next()?;
    if types.next().is_some() || !is_handle(inner) {
        return None;
    }
    if segment.ident == "Option" {
        Some(Shape::Optional)
    } else if segment.ident == "Vec" {
        Some(Shape::Many)
    } else {
        None
    }
}
