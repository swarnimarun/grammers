// Copyright 2020 - developers of the `grammers` project.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Code to generate Rust's `enum`'s from TL definitions.

use crate::grouper;
use crate::metadata::Metadata;
use crate::rustifier::{rusty_class_name, rusty_namespaced_type_name};
use grammers_tl_parser::tl::{Category, Definition};
use std::io::{self, Write};

/// Writes an enumeration listing all types such as the following rust code:
///
/// ```
/// pub enum Name {
///     Variant(crate::types::Name),
/// }
/// ```
fn write_enum<W: Write>(
    file: &mut W,
    indent: &str,
    name: &str,
    type_defs: &Vec<&Definition>,
    metadata: &Metadata,
) -> io::Result<()> {
    if cfg!(feature = "impl-debug") {
        writeln!(file, "{}#[derive(Debug)]", indent)?;
    }

    writeln!(file, "{}pub enum {} {{", indent, rusty_class_name(name))?;
    for d in type_defs.iter() {
        write!(file, "{}    {}(", indent, rusty_class_name(&d.name))?;

        if metadata.is_recursive_def(d) {
            write!(file, "Box<")?;
        }
        write!(file, "{}", rusty_namespaced_type_name(&d))?;
        if metadata.is_recursive_def(d) {
            write!(file, ">")?;
        }

        writeln!(file, "),")?;
    }
    writeln!(file, "{}}}", indent)?;
    Ok(())
}

/// Defines the `impl Serializable` corresponding to the type definitions:
///
/// ```
/// impl crate::Serializable for Name {
///     fn serialize<B: std::io::Write>(&self, buf: &mut B) -> std::io::Result<()> {
///         use crate::Identifiable;
///         match self {
///             Self::Variant(x) => {
///                 crate::types::Name::CONSTRUCTOR_ID.serialize(buf)?;
///                 x.serialize(buf)
///             },
///         }
///     }
/// }
/// ```
fn write_serializable<W: Write>(
    file: &mut W,
    indent: &str,
    name: &str,
    type_defs: &Vec<&Definition>,
    _metadata: &Metadata,
) -> io::Result<()> {
    writeln!(
        file,
        "{}impl crate::Serializable for {} {{",
        indent,
        rusty_class_name(name)
    )?;
    writeln!(
        file,
        "{}    fn serialize<B: std::io::Write>(&self, buf: &mut B) -> std::io::Result<()> {{",
        indent
    )?;

    if type_defs.is_empty() {
        writeln!(file, "{}        Ok(())", indent)?;
    } else {
        writeln!(file, "{}        use crate::Identifiable;", indent)?;
        writeln!(file, "{}        match self {{", indent)?;
        for d in type_defs.iter() {
            writeln!(
                file,
                "{}            Self::{}(x) => {{",
                indent,
                rusty_class_name(&d.name)
            )?;
            writeln!(
                file,
                "{}                {}::CONSTRUCTOR_ID.serialize(buf)?;",
                indent,
                rusty_namespaced_type_name(&d)
            )?;
            writeln!(file, "{}                x.serialize(buf)", indent)?;
            writeln!(file, "{}            }},", indent)?;
        }
        writeln!(file, "{}        }}", indent)?;
    }
    writeln!(file, "{}    }}", indent)?;
    writeln!(file, "{}}}", indent)?;
    Ok(())
}

/// Defines the `impl Deserializable` corresponding to the type definitions:
///
/// ```
/// impl crate::Deserializable for Name {
///     fn deserialize<B: std::io::Read>(buf: &mut B) -> std::io::Result<Self> {
///         use crate::Identifiable;
///         Ok(match u32::deserialize(buf)? {
///             crate::types::Name::CONSTRUCTOR_ID => Self::Variant(crate::types::Name::deserialize(buf)?),
///             _ => return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, ...)),
///         })
///     }
/// }
/// ```
fn write_deserializable<W: Write>(
    file: &mut W,
    indent: &str,
    name: &str,
    type_defs: &Vec<&Definition>,
    metadata: &Metadata,
) -> io::Result<()> {
    writeln!(
        file,
        "{}impl crate::Deserializable for {} {{",
        indent,
        rusty_class_name(name)
    )?;
    writeln!(
        file,
        "{}    fn deserialize<B: std::io::Read>(buf: &mut B) -> std::io::Result<Self> {{",
        indent
    )?;
    writeln!(file, "{}        use crate::Identifiable;", indent)?;
    writeln!(file, "{}        let id = u32::deserialize(buf)?;", indent)?;
    writeln!(file, "{}        Ok(match id {{", indent)?;
    for d in type_defs.iter() {
        write!(
            file,
            "{}            {}::CONSTRUCTOR_ID => Self::{}(",
            indent,
            rusty_namespaced_type_name(&d),
            rusty_class_name(&d.name),
        )?;

        if metadata.is_recursive_def(d) {
            write!(file, "Box::new(")?;
        }
        write!(
            file,
            "{}::deserialize(buf)?",
            rusty_namespaced_type_name(&d)
        )?;
        if metadata.is_recursive_def(d) {
            write!(file, ")")?;
        }
        writeln!(file, "),")?;
    }
    writeln!(
        file,
        "{}            _ => return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, \
         crate::errors::UnexpectedConstructor {{ id }})),",
        indent
    )?;
    writeln!(file, "{}        }})", indent)?;
    writeln!(file, "{}    }}", indent)?;
    writeln!(file, "{}}}", indent)?;
    Ok(())
}

/// Defines the `impl From` corresponding to the definition:
///
/// ```
/// impl impl From<Name> for Enum {
/// }
/// ```
fn write_impl_from<W: Write>(
    file: &mut W,
    indent: &str,
    name: &str,
    type_defs: &Vec<&Definition>,
    metadata: &Metadata,
) -> io::Result<()> {
    for def in type_defs.iter() {
        writeln!(
            file,
            "{}impl From<{}> for {} {{",
            indent,
            rusty_namespaced_type_name(&def),
            rusty_class_name(name),
        )?;
        writeln!(
            file,
            "{}    fn from(x: {}) -> Self {{",
            indent,
            rusty_namespaced_type_name(&def),
        )?;
        writeln!(
            file,
            "{}        {cls}::{variant}({box_}x{paren})",
            indent,
            cls = rusty_class_name(name),
            box_ = if metadata.is_recursive_def(def) {
                "Box::new("
            } else {
                ""
            },
            variant = rusty_class_name(&def.name),
            paren = if metadata.is_recursive_def(def) {
                ")"
            } else {
                ""
            },
        )?;
        writeln!(file, "{}    }}", indent)?;
        writeln!(file, "{}}}", indent)?;
    }
    Ok(())
}

/// Writes an entire definition as Rust code (`enum` and `impl`).
fn write_definition<W: Write>(
    file: &mut W,
    indent: &str,
    name: &str,
    type_defs: &Vec<&Definition>,
    metadata: &Metadata,
) -> io::Result<()> {
    // TODO move type_defs into metadata
    write_enum(file, indent, name, type_defs, metadata)?;
    write_serializable(file, indent, name, type_defs, metadata)?;
    write_deserializable(file, indent, name, type_defs, metadata)?;
    if cfg!(feature = "impl-from-type") {
        write_impl_from(file, indent, name, type_defs, metadata)?;
    }
    Ok(())
}

/// Write the entire module dedicated to enums.
pub(crate) fn write_enums_mod<W: Write>(
    mut file: &mut W,
    definitions: &Vec<Definition>,
    metadata: &Metadata,
) -> io::Result<()> {
    // Begin outermost mod
    write!(
        file,
        "\
         /// This module contains all of the boxed types, each\n\
         /// represented by a `enum`. All of them implement\n\
         /// [`Serializable`] and [`Deserializable`].\n\
         ///\n\
         /// [`Serializable`]: /grammers_tl_types/trait.Serializable.html\n\
         /// [`Deserializable`]: /grammers_tl_types/trait.Deserializable.html\n\
         #[allow(clippy::large_enum_variant)]\n\
         pub mod enums {{\n\
         "
    )?;

    let grouped = grouper::group_types_by_ns(definitions);
    let mut sorted_keys: Vec<&Option<String>> = grouped.keys().collect();
    sorted_keys.sort();
    for key in sorted_keys.into_iter() {
        // Begin possibly inner mod
        let indent = if let Some(ns) = key {
            writeln!(file, "    #[allow(clippy::large_enum_variant)]")?;
            writeln!(file, "    pub mod {} {{", ns)?;
            "        "
        } else {
            "    "
        };

        for name in grouped[key].iter() {
            let type_defs: Vec<&Definition> = definitions
                .into_iter()
                .filter(|d| {
                    d.category == Category::Types
                        && d.ty.namespace.get(0) == key.as_ref()
                        && d.ty.name == **name
                })
                .collect();

            assert!(!type_defs.is_empty(), "type defs should not be empty");
            write_definition(&mut file, indent, name, &type_defs, metadata)?;
        }

        // End possibly inner mod
        if key.is_some() {
            writeln!(file, "    }}")?;
        }
    }

    // End outermost mod
    writeln!(file, "}}")
}
