// SPDX-FileCopyrightText: 2022 Klarälvdalens Datakonsult AB, a KDAB Group company <info@kdab.com>
// SPDX-FileContributor: Andrew Hayzen <andrew.hayzen@kdab.com>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

mod constructor;
pub mod cxxqttype;
pub mod externcxxqt;
pub mod fragment;
pub mod inherit;
pub mod locking;
pub mod method;
pub mod property;
pub mod qenum;
pub mod qnamespace;
pub mod qobject;
pub mod signal;
pub mod threading;

mod utils;

use std::collections::BTreeSet;

use crate::generator::cpp::fragment::CppNamedType;
use crate::naming::cpp::syn_type_to_cpp_type;
use crate::naming::TypeNames;
use crate::{generator::structuring, parser::Parser};
use externcxxqt::GeneratedCppExternCxxQtBlocks;
use qobject::GeneratedCppQObject;
use syn::spanned::Spanned;
use syn::{Error, FnArg, ForeignItemFn, Pat, PatIdent, PatType, Result};

/// Representation of the generated C++ code for a group of QObjects
pub struct GeneratedCppBlocks {
    /// Forward declarations that aren't associated with any QObjects (e.g. "free" qenums).
    pub forward_declares: Vec<String>,
    /// Additional includes for the CXX bridge
    pub includes: BTreeSet<String>,
    /// Stem of the CXX header to include
    pub cxx_file_stem: String,
    /// Generated QObjects
    pub qobjects: Vec<GeneratedCppQObject>,
    /// Generated extern C++Qt blocks
    pub extern_cxx_qt: Vec<GeneratedCppExternCxxQtBlocks>,
}

impl GeneratedCppBlocks {
    /// Create a [GeneratedCppBlocks] from the given [Parser] object
    pub fn from(parser: &Parser) -> Result<GeneratedCppBlocks> {
        let structures = structuring::Structures::new(&parser.cxx_qt_data)?;

        let mut includes = BTreeSet::new();

        let mut forward_declares: Vec<_> = parser
            .cxx_qt_data
            .qnamespaces
            .iter()
            .map(|parsed_qnamespace| qnamespace::generate(parsed_qnamespace, &mut includes))
            .collect();
        forward_declares.extend(
            parser
                .cxx_qt_data
                .qenums
                .iter()
                .map(|parsed_qenum| qenum::generate_declaration(parsed_qenum, &mut includes)),
        );
        Ok(GeneratedCppBlocks {
            forward_declares,
            includes,
            cxx_file_stem: parser.cxx_file_stem.clone(),
            qobjects: structures
                .qobjects
                .iter()
                .map(|qobject| GeneratedCppQObject::from(qobject, &parser.type_names))
                .collect::<Result<Vec<GeneratedCppQObject>>>()?,
            extern_cxx_qt: externcxxqt::generate(
                &parser.cxx_qt_data.extern_cxxqt_blocks,
                &parser.type_names,
            )?,
        })
    }
}

/// Returns a vector of the names and types ([CppNamedType] of the parameters of this method, used in cpp generation step
pub fn get_cpp_params(method: &ForeignItemFn, type_names: &TypeNames) -> Result<Vec<CppNamedType>> {
    method
        .sig
        .inputs
        .iter()
        .map(|input| {
            // Match parameters to extract their idents
            if let FnArg::Typed(PatType { pat, ty, .. }) = input {
                let ident = if let Pat::Ident(PatIdent { ident, .. }) = &**pat {
                    ident
                } else {
                    return Err(Error::new(input.span(), "Unknown pattern for type"));
                };

                // If the name of the argument is self then ignore,
                // as this is likely the self: Pin<T>
                if ident == "self" {
                    Ok(None)
                } else {
                    Ok(Some(CppNamedType {
                        ident: ident.to_string(),
                        ty: syn_type_to_cpp_type(ty, type_names)?,
                    }))
                }
            } else {
                Ok(None)
            }
        })
        .filter_map(|result| result.map_or_else(|e| Some(Err(e)), |v| v.map(Ok)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::parser::Parser;
    use syn::{parse_quote, ItemMod};

    #[test]
    fn test_generated_cpp_blocks() {
        let module: ItemMod = parse_quote! {
            #[cxx_qt::bridge]
            mod ffi {
                extern "RustQt" {
                    #[qobject]
                    type MyObject = super::MyObjectRust;
                }
            }
        };
        let parser = Parser::from(module).unwrap();

        let cpp = GeneratedCppBlocks::from(&parser).unwrap();
        assert_eq!(cpp.cxx_file_stem, "ffi");
        assert_eq!(cpp.qobjects.len(), 1);
        assert_eq!(cpp.qobjects[0].name.namespace(), None);
    }

    #[test]
    fn test_generated_cpp_blocks_cxx_file_stem() {
        let module: ItemMod = parse_quote! {
            #[cxx_qt::bridge(cxx_file_stem = "my_object")]
            mod ffi {
                extern "RustQt" {
                    #[qobject]
                    type MyObject = super::MyObjectRust;
                }
            }
        };
        let parser = Parser::from(module).unwrap();

        let cpp = GeneratedCppBlocks::from(&parser).unwrap();
        assert_eq!(cpp.cxx_file_stem, "my_object");
        assert_eq!(cpp.qobjects.len(), 1);
        assert_eq!(cpp.qobjects[0].name.namespace(), None);
    }

    #[test]
    fn test_generated_cpp_blocks_namespace() {
        let module: ItemMod = parse_quote! {
            #[cxx_qt::bridge(namespace = "cxx_qt")]
            mod ffi {
                extern "RustQt" {
                    #[qobject]
                    type MyObject = super::MyObjectRust;
                }
            }
        };
        let parser = Parser::from(module).unwrap();

        let cpp = GeneratedCppBlocks::from(&parser).unwrap();
        assert_eq!(cpp.qobjects[0].name.namespace(), Some("cxx_qt"));
    }
}
