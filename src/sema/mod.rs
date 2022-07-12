use self::{
    functions::{resolve_params, resolve_returns},
    symtable::Symtable,
    tags::parse_doccomments,
    variables::variable_decl,
};
use crate::file_resolver::{FileResolver, ResolvedFile};
use crate::sema::unused_variable::{check_unused_events, check_unused_namespace_variables};
use num_bigint::BigInt;
use solang_parser::{parse, pt};
use std::ffi::OsStr;

mod address;
pub mod ast;
pub mod builtin;
pub(crate) mod contracts;
pub mod diagnostics;
mod dotgraphviz;
pub(crate) mod eval;
pub(crate) mod expression;
mod file;
mod format;
mod functions;
mod mutability;
mod namespace;
mod statements;
pub mod symtable;
pub mod tags;
mod tests;
mod types;
mod unused_variable;
mod using;
mod variables;
pub(crate) mod yul;

pub type ArrayDimension = Option<(pt::Loc, BigInt)>;

// small prime number
pub const SOLANA_BUCKET_SIZE: u64 = 251;
pub const SOLANA_SPARSE_ARRAY_SIZE: u64 = 1024;

/// Load a file file from the cache, parse and resolve it. The file must be present in
/// the cache.
pub fn sema(file: &ResolvedFile, resolver: &mut FileResolver, ns: &mut ast::Namespace) {
    sema_file(file, resolver, ns);

    if !ns.diagnostics.any_errors() {
        // Checks for unused variables
        check_unused_namespace_variables(ns);
        check_unused_events(ns);
    }
}

/// Parse and resolve a file and its imports in a recursive manner.
fn sema_file(file: &ResolvedFile, resolver: &mut FileResolver, ns: &mut ast::Namespace) {
    let file_no = ns.files.len();

    let (source_code, file_cache_no) = resolver.get_file_contents_and_number(&file.full_path);

    ns.files.push(ast::File::new(
        file.full_path.clone(),
        &source_code,
        file_cache_no,
    ));

    let (pt, _) = match parse(&source_code, file_no) {
        Ok(s) => s,
        Err(errors) => {
            ns.diagnostics.extend(errors);

            return;
        }
    };

    // We need to iterate over the parsed contracts a few times, so create a temporary vector
    // This should be done before the contract types are created so the contract type numbers line up
    let contracts_to_resolve =
        pt.0.iter()
            .filter_map(|part| {
                if let pt::SourceUnitPart::ContractDefinition(def) = part {
                    Some(def)
                } else {
                    None
                }
            })
            .enumerate()
            .map(|(no, def)| (no + ns.contracts.len(), def.as_ref()))
            .collect::<Vec<(usize, &pt::ContractDefinition)>>();

    // first resolve all the types we can find
    let fields = types::resolve_typenames(&pt, file_no, ns);

    // resolve pragmas and imports
    for part in &pt.0 {
        match part {
            pt::SourceUnitPart::PragmaDirective(loc, name, value) => {
                resolve_pragma(loc, name, value, ns);
            }
            pt::SourceUnitPart::ImportDirective(import) => {
                resolve_import(import, Some(file), file_no, resolver, ns);
            }
            _ => (),
        }
    }

    contracts::resolve_base_contracts(&contracts_to_resolve, file_no, ns);

    // once all the types are resolved, we can resolve the structs and events. This is because
    // struct fields or event fields can have types defined elsewhere.
    types::resolve_fields(fields, file_no, ns);

    // resolve functions/constants outside of contracts
    let mut resolve_bodies = Vec::new();
    let mut doccomments = Vec::new();

    for part in &pt.0 {
        match part {
            pt::SourceUnitPart::FunctionDefinition(func) => {
                let tags = parse_doccomments(&doccomments);
                doccomments.clear();

                if let Some(func_no) = functions::function(func, file_no, &tags, ns) {
                    resolve_bodies.push((func_no, func));
                }
            }
            pt::SourceUnitPart::VariableDefinition(var) => {
                let tags = parse_doccomments(&doccomments);
                doccomments.clear();

                variable_decl(None, var, file_no, &tags, None, ns, &mut Symtable::new());
            }
            pt::SourceUnitPart::DocComment(doccomment) => doccomments.push(doccomment),
            _ => doccomments.clear(),
        }
    }

    // Now we can resolve the global using directives
    for part in &pt.0 {
        if let pt::SourceUnitPart::Using(using) = part {
            if let Ok(using) = using::using_decl(using, file_no, None, ns) {
                ns.using.push(using);
            }
        }
    }

    // now resolve the contracts
    contracts::resolve(&contracts_to_resolve, file_no, ns);

    // now we can resolve the body of functions outside of contracts
    for (func_no, func) in resolve_bodies {
        let _ = statements::resolve_function_body(func, file_no, None, func_no, ns);
    }

    // check for stray semi colons
    for part in &pt.0 {
        match part {
            pt::SourceUnitPart::StraySemicolon(loc) => {
                ns.diagnostics
                    .push(ast::Diagnostic::error(*loc, "stray semicolon".to_string()));
            }
            pt::SourceUnitPart::ContractDefinition(contract) => {
                for part in &contract.parts {
                    if let pt::ContractPart::StraySemicolon(loc) = part {
                        ns.diagnostics
                            .push(ast::Diagnostic::error(*loc, "stray semicolon".to_string()));
                    }
                }
            }
            _ => (),
        }
    }

    // now check state mutability for all contracts
    mutability::mutability(file_no, ns);
}

/// Find import file, resolve it by calling sema and add it to the namespace
fn resolve_import(
    import: &pt::Import,
    parent: Option<&ResolvedFile>,
    file_no: usize,
    resolver: &mut FileResolver,
    ns: &mut ast::Namespace,
) {
    let filename = match import {
        pt::Import::Plain(f, _) => f,
        pt::Import::GlobalSymbol(f, _, _) => f,
        pt::Import::Rename(f, _, _) => f,
    };

    let os_filename = OsStr::new(&filename.string);

    let import_file_no = if let Some(builtin_file_no) = ns
        .files
        .iter()
        .position(|file| file.cache_no.is_none() && file.path == os_filename)
    {
        // import "solana"
        builtin_file_no
    } else {
        match resolver.resolve_file(parent, os_filename) {
            Err(message) => {
                ns.diagnostics
                    .push(ast::Diagnostic::error(filename.loc, message));

                return;
            }
            Ok(file) => {
                if !ns.files.iter().any(|f| f.path == file.full_path) {
                    sema_file(&file, resolver, ns);

                    // give up if we failed
                    if ns.diagnostics.any_errors() {
                        return;
                    }
                }

                ns.files
                    .iter()
                    .position(|f| f.path == file.full_path)
                    .expect("import should be loaded by now")
            }
        }
    };

    match import {
        pt::Import::Rename(_, renames, _) => {
            for (from, rename_to) in renames {
                if let Some(import) =
                    ns.variable_symbols
                        .get(&(import_file_no, None, from.name.to_owned()))
                {
                    let import = import.clone();

                    let symbol = rename_to.as_ref().unwrap_or(from);

                    // Only add symbol if it does not already exist with same definition
                    if let Some(existing) =
                        ns.variable_symbols
                            .get(&(file_no, None, symbol.name.clone()))
                    {
                        if existing == &import {
                            continue;
                        }
                    }

                    ns.add_symbol(file_no, None, symbol, import);
                } else if let Some(import) =
                    ns.function_symbols
                        .get(&(import_file_no, None, from.name.to_owned()))
                {
                    let import = import.clone();

                    let symbol = rename_to.as_ref().unwrap_or(from);

                    // Only add symbol if it does not already exist with same definition
                    if let Some(existing) =
                        ns.function_symbols
                            .get(&(file_no, None, symbol.name.clone()))
                    {
                        if existing == &import {
                            continue;
                        }
                    }

                    ns.add_symbol(file_no, None, symbol, import);
                } else {
                    ns.diagnostics.push(ast::Diagnostic::error(
                        from.loc,
                        format!(
                            "import '{}' does not export '{}'",
                            filename.string, from.name
                        ),
                    ));
                }
            }
        }
        pt::Import::Plain(..) => {
            // find all the exports for the file
            let exports = ns
                .variable_symbols
                .iter()
                .filter_map(|((file_no, contract_no, id), symbol)| {
                    if *file_no == import_file_no {
                        Some((id.clone(), *contract_no, symbol.clone()))
                    } else {
                        None
                    }
                })
                .collect::<Vec<(String, Option<usize>, ast::Symbol)>>();

            for (name, contract_no, symbol) in exports {
                let new_symbol = pt::Identifier {
                    name: name.clone(),
                    loc: filename.loc,
                };

                // Only add symbol if it does not already exist with same definition
                if let Some(existing) =
                    ns.variable_symbols
                        .get(&(file_no, contract_no, name.clone()))
                {
                    if existing == &symbol {
                        continue;
                    }
                }

                ns.add_symbol(file_no, contract_no, &new_symbol, symbol);
            }

            let exports = ns
                .function_symbols
                .iter()
                .filter_map(|((file_no, contract_no, id), symbol)| {
                    if *file_no == import_file_no && contract_no.is_none() {
                        Some((id.clone(), symbol.clone()))
                    } else {
                        None
                    }
                })
                .collect::<Vec<(String, ast::Symbol)>>();

            for (name, symbol) in exports {
                let new_symbol = pt::Identifier {
                    name: name.clone(),
                    loc: filename.loc,
                };

                // Only add symbol if it does not already exist with same definition
                if let Some(existing) = ns.function_symbols.get(&(file_no, None, name.clone())) {
                    if existing == &symbol {
                        continue;
                    }
                }

                ns.add_symbol(file_no, None, &new_symbol, symbol);
            }
        }
        pt::Import::GlobalSymbol(_, symbol, _) => {
            ns.add_symbol(
                file_no,
                None,
                symbol,
                ast::Symbol::Import(symbol.loc, import_file_no),
            );
        }
    }
}

/// Resolve pragma. We don't do anything with pragmas for now
fn resolve_pragma(
    loc: &pt::Loc,
    name: &pt::Identifier,
    value: &pt::StringLiteral,
    ns: &mut ast::Namespace,
) {
    if name.name == "solidity" {
        ns.diagnostics.push(ast::Diagnostic::debug(
            *loc,
            "pragma 'solidity' is ignored".to_string(),
        ));
    } else if name.name == "experimental" && value.string == "ABIEncoderV2" {
        ns.diagnostics.push(ast::Diagnostic::debug(
            *loc,
            "pragma 'experimental' with value 'ABIEncoderV2' is ignored".to_string(),
        ));
    } else if name.name == "abicoder" && value.string == "v2" {
        ns.diagnostics.push(ast::Diagnostic::debug(
            *loc,
            "pragma 'abicoder' with value 'v2' is ignored".to_string(),
        ));
    } else {
        ns.diagnostics.push(ast::Diagnostic::warning(
            *loc,
            format!(
                "unknown pragma '{}' with value '{}' ignored",
                name.name, value.string
            ),
        ));
    }
}

pub trait Recurse {
    type ArgType;
    /// recurse over a structure
    fn recurse<T>(&self, cx: &mut T, f: fn(expr: &Self::ArgType, ctx: &mut T) -> bool);
}
