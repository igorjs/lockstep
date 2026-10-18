// SPDX-License-Identifier: Apache-2.0
//! The schema text `#[derive(Message)]` writes into `Message::SCHEMA`: the type's name, version
//! and shape as JSON, from the field types as written (decision 0062).

use quote::ToTokens;
use syn::{Data, DeriveInput, Fields, GenericParam};

/// A JSON value. Objects keep their keys in the order given, so the text is the same every time.
enum Json {
    Text(String),
    Number(u64),
    Array(Vec<Json>),
    Object(Vec<(&'static str, Json)>),
}

impl Json {
    fn is_scalar(&self) -> bool {
        matches!(self, Json::Text(_) | Json::Number(_))
    }

    /// Objects of scalars sit on one line; everything else has a line for each member.
    fn render(&self, depth: usize, out: &mut String) {
        let indent = "  ".repeat(depth + 1);
        let closing = "  ".repeat(depth);
        match self {
            Json::Text(text) => {
                out.push('"');
                for character in text.chars() {
                    match character {
                        '"' => out.push_str("\\\""),
                        '\\' => out.push_str("\\\\"),
                        other => out.push(other),
                    }
                }
                out.push('"');
            }
            Json::Number(number) => out.push_str(&number.to_string()),
            Json::Array(items) if items.is_empty() => out.push_str("[]"),
            Json::Array(items) => {
                out.push_str("[\n");
                for (index, item) in items.iter().enumerate() {
                    out.push_str(&indent);
                    item.render(depth + 1, out);
                    out.push_str(if index + 1 < items.len() { ",\n" } else { "\n" });
                }
                out.push_str(&closing);
                out.push(']');
            }
            Json::Object(members) if members.iter().all(|(_, value)| value.is_scalar()) => {
                out.push_str("{ ");
                for (index, (key, value)) in members.iter().enumerate() {
                    if index > 0 {
                        out.push_str(", ");
                    }
                    out.push_str(&format!("\"{key}\": "));
                    value.render(depth, out);
                }
                out.push_str(" }");
            }
            Json::Object(members) => {
                out.push_str("{\n");
                for (index, (key, value)) in members.iter().enumerate() {
                    out.push_str(&format!("{indent}\"{key}\": "));
                    value.render(depth + 1, out);
                    out.push_str(if index + 1 < members.len() {
                        ",\n"
                    } else {
                        "\n"
                    });
                }
                out.push_str(&closing);
                out.push('}');
            }
        }
    }
}

/// A type or parameter as written, without the spaces the token printer puts between tokens: a
/// space stays only between two words, as in `&mut T`.
fn written(tokens: impl ToTokens) -> String {
    let spaced = tokens.to_token_stream().to_string();
    let characters: Vec<char> = spaced.chars().collect();
    let word = |character: char| character.is_alphanumeric() || character == '_';
    let mut text = String::new();
    for (index, character) in characters.iter().enumerate() {
        if *character == ' ' {
            let before = index.checked_sub(1).map(|at| characters[at]);
            let after = characters.get(index + 1).copied();
            if !matches!((before, after), (Some(a), Some(b)) if word(a) && word(b)) {
                continue;
            }
        }
        text.push(*character);
    }
    text
}

fn form(fields: &Fields) -> &'static str {
    match fields {
        Fields::Named(_) => "named",
        Fields::Unnamed(_) => "tuple",
        Fields::Unit => "unit",
    }
}

fn fields(fields: &Fields) -> Json {
    Json::Array(
        fields
            .iter()
            .enumerate()
            .map(|(index, field)| {
                let name = match &field.ident {
                    Some(name) => name.to_string(),
                    None => index.to_string(),
                };
                Json::Object(vec![
                    ("name", Json::Text(name)),
                    ("type", Json::Text(written(&field.ty))),
                ])
            })
            .collect(),
    )
}

/// The pretty printed schema of a message type with the given version.
pub fn schema_text(input: &DeriveInput, version: u64) -> String {
    let mut members = vec![
        ("name", Json::Text(input.ident.to_string())),
        ("version", Json::Number(version)),
    ];
    if !input.generics.params.is_empty() {
        members.push((
            "generics",
            Json::Array(
                input
                    .generics
                    .params
                    .iter()
                    .map(|parameter| match parameter {
                        GenericParam::Type(parameter) => Json::Text(parameter.ident.to_string()),
                        GenericParam::Lifetime(parameter) => {
                            Json::Text(written(&parameter.lifetime))
                        }
                        GenericParam::Const(parameter) => Json::Text(parameter.ident.to_string()),
                    })
                    .collect(),
            ),
        ));
    }
    match &input.data {
        Data::Struct(data) => {
            members.push(("kind", Json::Text("struct".to_string())));
            members.push(("form", Json::Text(form(&data.fields).to_string())));
            members.push(("fields", fields(&data.fields)));
        }
        Data::Enum(data) => {
            members.push(("kind", Json::Text("enum".to_string())));
            members.push((
                "variants",
                Json::Array(
                    data.variants
                        .iter()
                        .enumerate()
                        .map(|(index, variant)| {
                            Json::Object(vec![
                                ("index", Json::Number(index as u64)),
                                ("name", Json::Text(variant.ident.to_string())),
                                ("form", Json::Text(form(&variant.fields).to_string())),
                                ("fields", fields(&variant.fields)),
                            ])
                        })
                        .collect(),
                ),
            ));
        }
        Data::Union(_) => {}
    }
    let mut text = String::new();
    Json::Object(members).render(0, &mut text);
    text
}
