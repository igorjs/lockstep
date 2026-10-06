// SPDX-License-Identifier: Apache-2.0
//! `#[derive(Message)]` for lockstep-core. Use it through `lockstep_core::Message`.

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as Tokens};
use quote::{format_ident, quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{parse_macro_input, Data, DeriveInput, Fields, Ident, LitInt, Type};

/// Implements `lockstep_core::Message` with the version from `#[message(version = N)]`, and
/// `lockstep_core::Indexable`: the kind is an enum's variant index (0 for a struct), and the
/// handles are every `Handle` a field holds, directly or inside `Option`, `Vec`, `Box`, an array,
/// a slice or a tuple, in field order. `#[message(version = N, manual_indexable)]` leaves
/// `Indexable` to a hand written implementation.
#[proc_macro_derive(Message, attributes(message))]
pub fn derive_message(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand(&input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

struct Settings {
    version: LitInt,
    manual_indexable: bool,
}

fn expand(input: &DeriveInput) -> syn::Result<Tokens> {
    let settings = settings(input)?;
    refuse_inner_attributes(input)?;
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
    let version = &settings.version;
    let message = quote! {
        impl #impl_generics ::lockstep_core::Message for #name #type_generics #where_clause {
            const VERSION: u32 = #version;
        }
    };
    if settings.manual_indexable {
        return Ok(message);
    }
    // One call-site identifier for the output list, so every use resolves to the parameter
    // whatever span the field types carry.
    let out = Ident::new("__lockstep_out", Span::call_site());
    let (kind, handles) = match &input.data {
        Data::Struct(data) => {
            let (pattern, pushes) = destructure(&data.fields, &out);
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
            if data.variants.is_empty() {
                // No value of an empty enum exists; dereferencing proves it to the compiler.
                (quote! { match *self {} }, quote! { match *self {} })
            } else {
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
                    let (pattern, pushes) = destructure(&variant.fields, &out);
                    quote! { Self::#variant_name #pattern => { #pushes } }
                });
                (
                    quote! { match self { #(#kinds)* } },
                    quote! { match self { #(#handle_arms)* } },
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
        #message

        impl #impl_generics ::lockstep_core::Indexable for #name #type_generics #where_clause {
            fn kind(&self) -> u16 {
                #kind
            }

            #[allow(unused_variables)]
            fn handles(&self, #out: &mut ::std::vec::Vec<::lockstep_core::Handle>) {
                #handles
            }
        }
    })
}

/// Reads `#[message(version = N)]`, which every message must carry exactly once, and the optional
/// `manual_indexable`.
fn settings(input: &DeriveInput) -> syn::Result<Settings> {
    let mut version: Option<LitInt> = None;
    let mut manual_indexable = false;
    for attribute in input
        .attrs
        .iter()
        .filter(|attribute| attribute.path().is_ident("message"))
    {
        attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident("version") {
                let value: LitInt = meta.value()?.parse()?;
                if !value.suffix().is_empty() {
                    return Err(syn::Error::new(
                        value.span(),
                        "write the version without a type suffix",
                    ));
                }
                value.base10_parse::<u32>()?;
                if version.replace(value).is_some() {
                    return Err(meta.error("the version is given twice"));
                }
                Ok(())
            } else if meta.path.is_ident("manual_indexable") {
                manual_indexable = true;
                Ok(())
            } else {
                Err(meta.error("expected `version = N` or `manual_indexable`"))
            }
        })?;
    }
    let version = version.ok_or_else(|| {
        syn::Error::new(
            input.ident.span(),
            "a message needs `#[message(version = N)]`: a shipped shape is never edited, so every change is a new version",
        )
    })?;
    Ok(Settings {
        version,
        manual_indexable,
    })
}

/// A version belongs to the whole message, so `#[message]` on a variant or a field is a mistake.
fn refuse_inner_attributes(input: &DeriveInput) -> syn::Result<()> {
    let refuse = |attributes: &[syn::Attribute]| match attributes
        .iter()
        .find(|attribute| attribute.path().is_ident("message"))
    {
        Some(attribute) => Err(syn::Error::new(
            attribute.span(),
            "`#[message]` belongs on the type: a version covers the whole message",
        )),
        None => Ok(()),
    };
    let fields: Vec<&syn::Field> = match &input.data {
        Data::Struct(data) => data.fields.iter().collect(),
        Data::Enum(data) => {
            for variant in &data.variants {
                refuse(&variant.attrs)?;
            }
            data.variants
                .iter()
                .flat_map(|variant| variant.fields.iter())
                .collect()
        }
        Data::Union(_) => Vec::new(),
    };
    for field in fields {
        refuse(&field.attrs)?;
    }
    Ok(())
}

/// A pattern binding every field, and the code that pushes the handles among them.
fn destructure(fields: &Fields, out: &Ident) -> (Tokens, Tokens) {
    let bindings: Vec<_> = (0..fields.len())
        .map(|index| format_ident!("__lockstep_field_{}", index))
        .collect();
    let mut depth = 0;
    let pushes = fields.iter().zip(&bindings).filter_map(|(field, binding)| {
        let span = field.ty.span();
        pushes_for(&field.ty, quote! { #binding }, out, &mut depth)
            .map(|code| quote_spanned! { span=> #code })
    });
    let pushes: Vec<_> = pushes.collect();
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

/// Code that pushes every handle inside `value` (a reference to a `field_type`), or `None` when
/// the type holds none the derive can see.
fn pushes_for(field_type: &Type, value: Tokens, out: &Ident, depth: &mut usize) -> Option<Tokens> {
    *depth += 1;
    let item = format_ident!("__lockstep_item_{}", *depth);
    match field_type {
        Type::Paren(inner) => pushes_for(&inner.elem, value, out, depth),
        Type::Group(inner) => pushes_for(&inner.elem, value, out, depth),
        Type::Array(array) => {
            let inner = pushes_for(&array.elem, quote! { #item }, out, depth)?;
            Some(quote! { for #item in #value.iter() { #inner } })
        }
        Type::Slice(slice) => {
            let inner = pushes_for(&slice.elem, quote! { #item }, out, depth)?;
            Some(quote! { for #item in #value.iter() { #inner } })
        }
        Type::Tuple(tuple) => {
            let names: Vec<_> = (0..tuple.elems.len())
                .map(|index| format_ident!("__lockstep_part_{}_{}", *depth, index))
                .collect();
            let parts: Vec<_> = tuple
                .elems
                .iter()
                .zip(&names)
                .filter_map(|(element, name)| pushes_for(element, quote! { #name }, out, depth))
                .collect();
            if parts.is_empty() {
                return None;
            }
            Some(quote! {
                let ( #(#names),* ) = #value;
                #(#parts)*
            })
        }
        Type::Path(path) if path.qself.is_none() => {
            let segment = path.path.segments.last()?;
            if segment.ident == "Handle" && segment.arguments.is_none() {
                return Some(quote! { #out.push(*#value); });
            }
            let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
                return None;
            };
            let mut types = arguments.args.iter().filter_map(|argument| match argument {
                syn::GenericArgument::Type(inner) => Some(inner),
                _ => None,
            });
            let inner_type = types.next()?;
            if types.next().is_some() {
                return None;
            }
            if segment.ident == "Option" {
                let inner = pushes_for(inner_type, quote! { #item }, out, depth)?;
                Some(quote! { if let ::std::option::Option::Some(#item) = #value { #inner } })
            } else if segment.ident == "Vec" {
                let inner = pushes_for(inner_type, quote! { #item }, out, depth)?;
                Some(quote! { for #item in #value.iter() { #inner } })
            } else if segment.ident == "Box" {
                let inner = pushes_for(inner_type, quote! { #item }, out, depth)?;
                Some(quote! { let #item = &**#value; #inner })
            } else {
                None
            }
        }
        _ => None,
    }
}
