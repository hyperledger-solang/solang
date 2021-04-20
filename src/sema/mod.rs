use crate::parser::{parse, pt};
use crate::Target;
use ast::Diagnostic;
use num_bigint::BigInt;
use num_traits::Signed;
use num_traits::Zero;
use std::collections::HashMap;

mod address;
pub mod ast;
pub mod builtin;
pub mod contracts;
pub mod diagnostics;
pub mod eval;
pub mod expression;
mod format;
mod functions;
mod mutability;
pub mod printer;
mod statements;
pub mod symtable;
pub mod tags;
mod types;
mod variables;

use self::contracts::visit_bases;
use self::eval::eval_const_number;
use self::expression::expression;
use self::functions::{resolve_params, resolve_returns};
use self::symtable::Symtable;
use self::variables::var_decl;
use crate::file_cache::{FileCache, ResolvedFile};

pub type ArrayDimension = Option<(pt::Loc, BigInt)>;

// small prime number
pub const SOLANA_BUCKET_SIZE: u64 = 251;
// The sizeof(struct account_data_header)
pub const SOLANA_FIRST_OFFSET: u64 = 16;
pub const SOLANA_SPARSE_ARRAY_SIZE: u64 = 1024;

/// Load a file file from the cache, parse and resolve it. The file must be present in
/// the cache. This function is recursive for imports.
pub fn sema(file: ResolvedFile, cache: &mut FileCache, ns: &mut ast::Namespace) {
    let file_no = ns.files.len();

    let source_code = cache.get_file_contents(&file.full_path);

    ns.files.push(file.full_path.clone());

    let pt = match parse(&source_code, file_no) {
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
            pt::SourceUnitPart::PragmaDirective(name, value) => {
                resolve_pragma(name, value, ns);
            }
            pt::SourceUnitPart::ImportDirective(import) => {
                resolve_import(import, Some(&file), file_no, cache, ns);
            }
            _ => (),
        }
    }

    // once all the types are resolved, we can resolve the structs and events. This is because
    // struct fields or event fields can have types defined elsewhere.
    types::resolve_fields(fields, file_no, ns);

    // give up if we failed
    if diagnostics::any_errors(&ns.diagnostics) {
        return;
    }

    // resolve functions/constants outside of contracts
    let mut resolve_bodies = Vec::new();

    for part in &pt.0 {
        match part {
            pt::SourceUnitPart::FunctionDefinition(func) => {
                if let Some(func_no) = functions::function(func, file_no, ns) {
                    resolve_bodies.push((func_no, func));
                }
            }
            pt::SourceUnitPart::VariableDefinition(var) => {
                var_decl(None, var, file_no, None, ns, &mut Symtable::new());
            }
            _ => (),
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
    mutability::mutablity(file_no, ns);
}

/// Find import file, resolve it by calling sema and add it to the namespace
fn resolve_import(
    import: &pt::Import,
    parent: Option<&ResolvedFile>,
    file_no: usize,
    cache: &mut FileCache,
    ns: &mut ast::Namespace,
) {
    let filename = match import {
        pt::Import::Plain(f) => f,
        pt::Import::GlobalSymbol(f, _) => f,
        pt::Import::Rename(f, _) => f,
    };

    let import_file_no = match cache.resolve_file(parent, &filename.string) {
        Err(message) => {
            ns.diagnostics
                .push(ast::Diagnostic::error(filename.loc, message));

            return;
        }
        Ok(file) => {
            if !ns.files.iter().any(|f| *f == file.full_path) {
                sema(file.clone(), cache, ns);

                // give up if we failed
                if diagnostics::any_errors(&ns.diagnostics) {
                    return;
                }
            }

            ns.files
                .iter()
                .position(|f| *f == file.full_path)
                .expect("import should be loaded by now")
        }
    };

    match import {
        pt::Import::Rename(_, renames) => {
            for (from, rename_to) in renames {
                if let Some(import) = ns
                    .symbols
                    .get(&(import_file_no, None, from.name.to_owned()))
                {
                    let import = import.clone();

                    let new_symbol = if let Some(to) = rename_to { to } else { from };

                    // Only add symbol if it does not already exist with same definition
                    if let Some(existing) =
                        ns.symbols.get(&(file_no, None, new_symbol.name.clone()))
                    {
                        if existing == &import {
                            continue;
                        }
                    }

                    ns.check_shadowing(file_no, None, new_symbol);

                    ns.add_symbol(file_no, None, new_symbol, import);
                } else {
                    ns.diagnostics.push(ast::Diagnostic::error(
                        from.loc,
                        format!(
                            "import ‘{}’ does not export ‘{}’",
                            filename.string,
                            from.name.to_string()
                        ),
                    ));
                }
            }
        }
        pt::Import::Plain(_) => {
            // find all the exports for the file
            let exports = ns
                .symbols
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
                if let Some(existing) = ns.symbols.get(&(file_no, contract_no, name.clone())) {
                    if existing == &symbol {
                        continue;
                    }
                }

                ns.check_shadowing(file_no, contract_no, &new_symbol);

                ns.add_symbol(file_no, contract_no, &new_symbol, symbol);
            }
        }
        pt::Import::GlobalSymbol(_, symbol) => {
            ns.check_shadowing(file_no, None, &symbol);

            ns.add_symbol(
                file_no,
                None,
                &symbol,
                ast::Symbol::Import(symbol.loc, import_file_no),
            );
        }
    }
}

/// Resolve pragma. We don't do anything with pragmas for now
fn resolve_pragma(name: &pt::Identifier, value: &pt::StringLiteral, ns: &mut ast::Namespace) {
    if name.name == "solidity" {
        ns.diagnostics.push(ast::Diagnostic::debug(
            pt::Loc(name.loc.0, name.loc.1, value.loc.2),
            "pragma ‘solidity’ is ignored".to_string(),
        ));
    } else if name.name == "experimental" && value.string == "ABIEncoderV2" {
        ns.diagnostics.push(ast::Diagnostic::debug(
            pt::Loc(name.loc.0, name.loc.1, value.loc.2),
            "pragma ‘experimental’ with value ‘ABIEncoderV2’ is ignored".to_string(),
        ));
    } else if name.name == "abicoder" && value.string == "v2" {
        ns.diagnostics.push(ast::Diagnostic::debug(
            pt::Loc(name.loc.0, name.loc.1, value.loc.2),
            "pragma ‘abicoder’ with value ‘v2’ is ignored".to_string(),
        ));
    } else {
        ns.diagnostics.push(ast::Diagnostic::warning(
            pt::Loc(name.loc.0, name.loc.1, value.loc.2),
            format!(
                "unknown pragma ‘{}’ with value ‘{}’ ignored",
                name.name, value.string
            ),
        ));
    }
}

impl ast::Namespace {
    /// Create a namespace and populate with the parameters for the target
    pub fn new(target: Target, address_length: usize, value_length: usize) -> Self {
        ast::Namespace {
            target,
            files: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            events: Vec::new(),
            contracts: Vec::new(),
            functions: Vec::new(),
            constants: Vec::new(),
            address_length,
            value_length,
            symbols: HashMap::new(),
            diagnostics: Vec::new(),
            next_id: 0,
            var_constants: HashMap::new(),
            hover_overrides: HashMap::new(),
        }
    }

    /// Add symbol to symbol table; either returns true for success, or adds an appropriate error
    pub fn add_symbol(
        &mut self,
        file_no: usize,
        contract_no: Option<usize>,
        id: &pt::Identifier,
        symbol: ast::Symbol,
    ) -> bool {
        if builtin::is_reserved(&id.name) {
            self.diagnostics.push(ast::Diagnostic::error(
                id.loc,
                format!("‘{}’ shadows name of a builtin", id.name.to_string()),
            ));

            return false;
        }

        if let Some(sym) = self
            .symbols
            .get(&(file_no, contract_no, id.name.to_owned()))
        {
            match sym {
                ast::Symbol::Contract(c, _) => {
                    self.diagnostics.push(ast::Diagnostic::error_with_note(
                        id.loc,
                        format!(
                            "{} is already defined as a contract name",
                            id.name.to_string()
                        ),
                        *c,
                        "location of previous definition".to_string(),
                    ));
                }
                ast::Symbol::Enum(c, _) => {
                    self.diagnostics.push(ast::Diagnostic::error_with_note(
                        id.loc,
                        format!("{} is already defined as an enum", id.name.to_string()),
                        *c,
                        "location of previous definition".to_string(),
                    ));
                }
                ast::Symbol::Struct(c, _) => {
                    self.diagnostics.push(ast::Diagnostic::error_with_note(
                        id.loc,
                        format!("{} is already defined as a struct", id.name.to_string()),
                        *c,
                        "location of previous definition".to_string(),
                    ));
                }
                ast::Symbol::Event(events) => {
                    self.diagnostics.push(ast::Diagnostic::error_with_note(
                        id.loc,
                        format!("{} is already defined as an event", id.name.to_string()),
                        events[0].0,
                        "location of previous definition".to_string(),
                    ));
                }
                ast::Symbol::Variable(c, _, _) => {
                    self.diagnostics.push(ast::Diagnostic::error_with_note(
                        id.loc,
                        format!(
                            "{} is already defined as a contract variable",
                            id.name.to_string()
                        ),
                        *c,
                        "location of previous definition".to_string(),
                    ));
                }
                ast::Symbol::Function(v) => {
                    let notes = v
                        .iter()
                        .map(|(pos, _)| ast::Note {
                            pos: *pos,
                            message: "location of previous definition".to_owned(),
                        })
                        .collect();

                    self.diagnostics.push(ast::Diagnostic::error_with_notes(
                        id.loc,
                        format!("{} is already defined as a function", id.name.to_string()),
                        notes,
                    ));
                }
                ast::Symbol::Import(loc, _) => {
                    self.diagnostics.push(ast::Diagnostic::error_with_note(
                        id.loc,
                        format!("{} is already defined as an import", id.name.to_string()),
                        *loc,
                        "location of previous definition".to_string(),
                    ));
                }
            }

            return false;
        }

        // if there is nothing on the contract level, try top-level scope if its not an event
        if let ast::Symbol::Event(_) = &symbol {
            // it's ok to have event of same name in contract and global scope
        } else if contract_no.is_some() {
            if let Some(sym) = self.symbols.get(&(file_no, None, id.name.to_owned())) {
                match sym {
                    ast::Symbol::Contract(c, _) => {
                        self.diagnostics.push(ast::Diagnostic::warning_with_note(
                            id.loc,
                            format!(
                                "{} is already defined as a contract name",
                                id.name.to_string()
                            ),
                            *c,
                            "location of previous definition".to_string(),
                        ));
                    }
                    ast::Symbol::Enum(c, _) => {
                        self.diagnostics.push(ast::Diagnostic::warning_with_note(
                            id.loc,
                            format!("{} is already defined as an enum", id.name),
                            *c,
                            "location of previous definition".to_string(),
                        ));
                    }
                    ast::Symbol::Struct(c, _) => {
                        self.diagnostics.push(ast::Diagnostic::warning_with_note(
                            id.loc,
                            format!("{} is already defined as a struct", id.name),
                            *c,
                            "location of previous definition".to_string(),
                        ));
                    }
                    ast::Symbol::Event(e) => {
                        self.diagnostics.push(ast::Diagnostic::warning_with_note(
                            id.loc,
                            format!("{} is already defined as an event", id.name),
                            e[0].0,
                            "location of previous definition".to_string(),
                        ));
                    }
                    ast::Symbol::Variable(c, _, _) => {
                        self.diagnostics.push(ast::Diagnostic::warning_with_note(
                            id.loc,
                            format!(
                                "{} is already defined as a contract variable",
                                id.name.to_string()
                            ),
                            *c,
                            "location of previous definition".to_string(),
                        ));
                    }
                    ast::Symbol::Function(v) => {
                        let notes = v
                            .iter()
                            .map(|(pos, _)| ast::Note {
                                pos: *pos,
                                message: "location of previous definition".to_owned(),
                            })
                            .collect();

                        self.diagnostics.push(ast::Diagnostic::warning_with_notes(
                            id.loc,
                            format!("{} is already defined as a function", id.name),
                            notes,
                        ));
                    }
                    ast::Symbol::Import(loc, _) => {
                        self.diagnostics.push(ast::Diagnostic::warning_with_note(
                            id.loc,
                            format!("{} is already defined as an import", id.name),
                            *loc,
                            "location of previous definition".to_string(),
                        ));
                    }
                }
            }
        }

        self.symbols
            .insert((file_no, contract_no, id.name.to_string()), symbol);

        true
    }

    /// Resolve enum by name
    pub fn resolve_enum(
        &self,
        file_no: usize,
        contract_no: Option<usize>,
        id: &pt::Identifier,
    ) -> Option<usize> {
        if let Some(ast::Symbol::Enum(_, n)) =
            self.symbols
                .get(&(file_no, contract_no, id.name.to_owned()))
        {
            return Some(*n);
        }

        if let Some(contract_no) = contract_no {
            if let Some(ast::Symbol::Enum(_, n)) = self.resolve_base_contract(contract_no, id) {
                return Some(*n);
            }

            if let Some(ast::Symbol::Enum(_, n)) =
                self.symbols.get(&(file_no, None, id.name.to_owned()))
            {
                return Some(*n);
            }
        }

        None
    }

    /// Resolve a contract name
    pub fn resolve_contract(&self, file_no: usize, id: &pt::Identifier) -> Option<usize> {
        if let Some(ast::Symbol::Contract(_, n)) =
            self.symbols.get(&(file_no, None, id.name.to_owned()))
        {
            return Some(*n);
        }

        None
    }

    /// Resolve an event. We should only be resolving events for emit statements
    pub fn resolve_event(
        &mut self,
        file_no: usize,
        contract_no: Option<usize>,
        expr: &pt::Expression,
        diagnostics: &mut Vec<ast::Diagnostic>,
    ) -> Result<Vec<usize>, ()> {
        let (namespace, id, dimensions) =
            self.expr_to_type(file_no, contract_no, expr, diagnostics)?;

        if !dimensions.is_empty() {
            diagnostics.push(ast::Diagnostic::decl_error(
                expr.loc(),
                "array type found where event type expected".to_string(),
            ));
            return Err(());
        }

        let id = match id {
            pt::Expression::Variable(id) => id,
            _ => {
                diagnostics.push(ast::Diagnostic::decl_error(
                    expr.loc(),
                    "expression found where event type expected".to_string(),
                ));
                return Err(());
            }
        };

        // if we are resolving an event name without namespace (so no explicit contract name
        // or import symbol), then we should look both in the current contract and global scope
        if namespace.is_empty() {
            let mut events = Vec::new();

            // If we're in a contract, then event can be defined in current contract or its bases
            if let Some(contract_no) = contract_no {
                for contract_no in visit_bases(contract_no, self).into_iter().rev() {
                    match self
                        .symbols
                        .get(&(file_no, Some(contract_no), id.name.to_owned()))
                    {
                        None => (),
                        Some(ast::Symbol::Event(ev)) => {
                            for (_, event_no) in ev {
                                events.push(*event_no);
                            }
                        }
                        sym => {
                            let error = ast::Namespace::wrong_symbol(sym, &id);

                            diagnostics.push(error);

                            return Err(());
                        }
                    }
                }
            }

            return match self.symbols.get(&(file_no, None, id.name.to_owned())) {
                None if events.is_empty() => {
                    diagnostics.push(ast::Diagnostic::decl_error(
                        id.loc,
                        format!("event ‘{}’ not found", id.name),
                    ));
                    Err(())
                }
                None => Ok(events),
                Some(ast::Symbol::Event(ev)) => {
                    for (_, event_no) in ev {
                        events.push(*event_no);
                    }
                    Ok(events)
                }
                sym => {
                    let error = ast::Namespace::wrong_symbol(sym, &id);

                    diagnostics.push(error);

                    Err(())
                }
            };
        }

        let s = self.resolve_namespace(namespace, file_no, contract_no, &id, diagnostics)?;

        if let Some(ast::Symbol::Event(events)) = s {
            Ok(events.iter().map(|(_, event_no)| *event_no).collect())
        } else {
            let error = ast::Namespace::wrong_symbol(s, &id);

            diagnostics.push(error);

            Err(())
        }
    }

    pub fn wrong_symbol(sym: Option<&ast::Symbol>, id: &pt::Identifier) -> ast::Diagnostic {
        match sym {
            None => ast::Diagnostic::decl_error(id.loc, format!("`{}' is not found", id.name)),
            Some(ast::Symbol::Enum(_, _)) => {
                ast::Diagnostic::decl_error(id.loc, format!("`{}' is an enum", id.name))
            }
            Some(ast::Symbol::Struct(_, _)) => {
                ast::Diagnostic::decl_error(id.loc, format!("`{}' is a struct", id.name))
            }
            Some(ast::Symbol::Event(_)) => {
                ast::Diagnostic::decl_error(id.loc, format!("`{}' is an event", id.name))
            }
            Some(ast::Symbol::Function(_)) => {
                ast::Diagnostic::decl_error(id.loc, format!("`{}' is a function", id.name))
            }
            Some(ast::Symbol::Contract(_, _)) => {
                ast::Diagnostic::decl_error(id.loc, format!("`{}' is a contract", id.name))
            }
            Some(ast::Symbol::Import(_, _)) => {
                ast::Diagnostic::decl_error(id.loc, format!("`{}' is an import", id.name))
            }
            Some(ast::Symbol::Variable(_, _, _)) => {
                ast::Diagnostic::decl_error(id.loc, format!("`{}' is a contract variable", id.name))
            }
        }
    }

    /// Does a parent contract have a variable defined with this name (recursive)
    fn resolve_base_contract(
        &self,
        contract_no: usize,
        id: &pt::Identifier,
    ) -> Option<&ast::Symbol> {
        for base in self.contracts[contract_no].bases.iter() {
            // find file this contract was defined in
            let file_no = self.contracts[base.contract_no].loc.0;

            if let Some(sym) =
                self.symbols
                    .get(&(file_no, Some(base.contract_no), id.name.to_owned()))
            {
                if let ast::Symbol::Variable(_, var_contract_no, var_no) = sym {
                    if *var_contract_no != Some(base.contract_no) {
                        return None;
                    }

                    let var = &self.contracts[base.contract_no].variables[*var_no];

                    if let pt::Visibility::Private(_) = var.visibility {
                        return None;
                    }
                }

                return Some(sym);
            } else {
                let res = self.resolve_base_contract(base.contract_no, id);

                if res.is_some() {
                    return res;
                }
            }
        }

        None
    }

    /// Resolve contract variable
    pub fn resolve_var(
        &self,
        file_no: usize,
        contract_no: Option<usize>,
        id: &pt::Identifier,
    ) -> Option<&ast::Symbol> {
        let mut s = self
            .symbols
            .get(&(file_no, contract_no, id.name.to_owned()));

        if let Some(contract_no) = contract_no {
            if s.is_none() {
                if let Some(sym) = self.resolve_base_contract(contract_no, id) {
                    s = Some(sym);
                }
            }
        }

        if s.is_none() {
            self.symbols.get(&(file_no, None, id.name.to_owned()))
        } else {
            s
        }
    }

    /// Check if an name would shadow an existing symbol
    pub fn check_shadowing(
        &mut self,
        file_no: usize,
        contract_no: Option<usize>,
        id: &pt::Identifier,
    ) {
        if builtin::is_reserved(&id.name) {
            self.diagnostics.push(ast::Diagnostic::warning(
                id.loc,
                format!("‘{}’ shadows name of a builtin", id.name.to_string()),
            ));
            return;
        }

        let mut s = self
            .symbols
            .get(&(file_no, contract_no, id.name.to_owned()));

        if s.is_none() {
            s = self.symbols.get(&(file_no, None, id.name.to_owned()));
        }

        match s {
            Some(ast::Symbol::Enum(loc, _)) => {
                self.diagnostics.push(ast::Diagnostic::warning_with_note(
                    id.loc,
                    format!("declaration of `{}' shadows enum definition", id.name),
                    *loc,
                    "previous definition of enum".to_string(),
                ));
            }
            Some(ast::Symbol::Struct(loc, _)) => {
                self.diagnostics.push(ast::Diagnostic::warning_with_note(
                    id.loc,
                    format!("declaration of `{}' shadows struct definition", id.name),
                    *loc,
                    "previous definition of struct".to_string(),
                ));
            }
            Some(ast::Symbol::Event(events)) => {
                let notes = events
                    .iter()
                    .map(|(pos, _)| ast::Note {
                        pos: *pos,
                        message: "previous definition of event".to_owned(),
                    })
                    .collect();

                self.diagnostics.push(ast::Diagnostic::warning_with_notes(
                    id.loc,
                    format!("declaration of `{}' shadows event definition", id.name),
                    notes,
                ));
            }
            Some(ast::Symbol::Function(v)) => {
                let notes = v
                    .iter()
                    .map(|(pos, _)| ast::Note {
                        pos: *pos,
                        message: "previous declaration of function".to_owned(),
                    })
                    .collect();
                self.diagnostics.push(ast::Diagnostic::warning_with_notes(
                    id.loc,
                    format!("declaration of ‘{}’ shadows function", id.name),
                    notes,
                ));
            }
            Some(ast::Symbol::Variable(loc, _, _)) => {
                self.diagnostics.push(ast::Diagnostic::warning_with_note(
                    id.loc,
                    format!("declaration of ‘{}’ shadows state variable", id.name),
                    *loc,
                    "previous declaration of state variable".to_string(),
                ));
            }
            Some(ast::Symbol::Contract(loc, _)) => {
                self.diagnostics.push(ast::Diagnostic::warning_with_note(
                    id.loc,
                    format!("declaration of ‘{}’ shadows contract name", id.name),
                    *loc,
                    "previous declaration of contract name".to_string(),
                ));
            }
            Some(ast::Symbol::Import(loc, _)) => {
                self.diagnostics.push(ast::Diagnostic::warning_with_note(
                    id.loc,
                    format!("declaration of ‘{}’ shadows import", id.name),
                    *loc,
                    "previous declaration of import".to_string(),
                ));
            }
            None => {}
        }
    }

    /// Resolve the parsed data type. The type can be a primitive, enum and also an arrays.
    /// The type for address payable is "address payable" used as a type, and "payable" when
    /// casting. So, we need to know what we are resolving for.
    pub fn resolve_type(
        &mut self,
        file_no: usize,
        contract_no: Option<usize>,
        casting: bool,
        id: &pt::Expression,
        diagnostics: &mut Vec<ast::Diagnostic>,
    ) -> Result<ast::Type, ()> {
        fn resolve_dimensions(
            ast_dimensions: &[Option<(pt::Loc, BigInt)>],
            diagnostics: &mut Vec<Diagnostic>,
        ) -> Result<Vec<Option<BigInt>>, ()> {
            let mut dimensions = Vec::new();

            for d in ast_dimensions.iter().rev() {
                if let Some((loc, n)) = d {
                    if n.is_zero() {
                        diagnostics.push(ast::Diagnostic::decl_error(
                            *loc,
                            "zero size array not permitted".to_string(),
                        ));
                        return Err(());
                    } else if n.is_negative() {
                        diagnostics.push(ast::Diagnostic::decl_error(
                            *loc,
                            "negative size of array declared".to_string(),
                        ));
                        return Err(());
                    }
                    dimensions.push(Some(n.clone()));
                } else {
                    dimensions.push(None);
                }
            }

            Ok(dimensions)
        }

        let (namespace, id, dimensions) =
            self.expr_to_type(file_no, contract_no, &id, diagnostics)?;

        if let pt::Expression::Type(_, ty) = &id {
            assert!(namespace.is_empty());

            let ty = match ty {
                pt::Type::Mapping(_, k, v) => {
                    let key = self.resolve_type(file_no, contract_no, false, k, diagnostics)?;
                    let value = self.resolve_type(file_no, contract_no, false, v, diagnostics)?;

                    match key {
                        ast::Type::Mapping(_, _) => {
                            diagnostics.push(ast::Diagnostic::decl_error(
                                k.loc(),
                                "key of mapping cannot be another mapping type".to_string(),
                            ));
                            return Err(());
                        }
                        ast::Type::Struct(_) => {
                            diagnostics.push(ast::Diagnostic::decl_error(
                                k.loc(),
                                "key of mapping cannot be struct type".to_string(),
                            ));
                            return Err(());
                        }
                        ast::Type::Array(_, _) => {
                            diagnostics.push(ast::Diagnostic::decl_error(
                                k.loc(),
                                "key of mapping cannot be array type".to_string(),
                            ));
                            return Err(());
                        }
                        _ => ast::Type::Mapping(Box::new(key), Box::new(value)),
                    }
                }
                pt::Type::Function {
                    params,
                    attributes,
                    returns,
                    trailing_attributes,
                } => {
                    let mut mutability: Option<pt::StateMutability> = None;
                    let mut visibility: Option<pt::Visibility> = None;

                    let mut success = true;

                    for a in attributes {
                        match a {
                            pt::FunctionAttribute::StateMutability(m) => {
                                if let Some(e) = &mutability {
                                    diagnostics.push(ast::Diagnostic::error_with_note(
                                        m.loc(),
                                        format!(
                                            "function type mutability redeclared `{}'",
                                            m.to_string()
                                        ),
                                        e.loc(),
                                        format!(
                                            "location of previous mutability declaration of `{}'",
                                            e.to_string()
                                        ),
                                    ));
                                    success = false;
                                    continue;
                                }

                                if let pt::StateMutability::Constant(loc) = m {
                                    diagnostics.push(ast::Diagnostic::warning(
                                        *loc,
                                        "‘constant’ is deprecated. Use ‘view’ instead".to_string(),
                                    ));

                                    mutability = Some(pt::StateMutability::View(*loc));
                                } else {
                                    mutability = Some(m.clone());
                                }
                            }
                            pt::FunctionAttribute::Visibility(v) => {
                                if let Some(e) = &visibility {
                                    diagnostics.push(ast::Diagnostic::error_with_note(
                                        v.loc(),
                                        format!(
                                            "function type visibility redeclared `{}'",
                                            v.to_string()
                                        ),
                                        e.loc(),
                                        format!(
                                            "location of previous visibility declaration of `{}'",
                                            e.to_string()
                                        ),
                                    ));
                                    success = false;
                                    continue;
                                }

                                visibility = Some(v.clone());
                            }
                            _ => unreachable!(),
                        }
                    }

                    let is_external = match visibility {
                        None | Some(pt::Visibility::Internal(_)) => false,
                        Some(pt::Visibility::External(_)) => true,
                        Some(v) => {
                            diagnostics.push(ast::Diagnostic::error(
                                v.loc(),
                                format!("function type cannot have visibility attribute `{}'", v),
                            ));
                            success = false;
                            false
                        }
                    };

                    let (params, params_success) = resolve_params(
                        params,
                        is_external,
                        file_no,
                        contract_no,
                        self,
                        diagnostics,
                    );

                    let (returns, returns_success) = resolve_returns(
                        returns,
                        is_external,
                        file_no,
                        contract_no,
                        self,
                        diagnostics,
                    );

                    // trailing attribute should not be there
                    // trailing visibility for contract variables should be removed already
                    for a in trailing_attributes {
                        match a {
                            pt::FunctionAttribute::StateMutability(m) => {
                                diagnostics.push(ast::Diagnostic::error(
                                    m.loc(),
                                    format!(
                                        "mutability `{}' cannot be declared after returns",
                                        m.to_string()
                                    ),
                                ));
                                success = false;
                            }
                            pt::FunctionAttribute::Visibility(v) => {
                                diagnostics.push(ast::Diagnostic::error(
                                    v.loc(),
                                    format!(
                                        "visibility `{}' cannot be declared after returns",
                                        v.to_string()
                                    ),
                                ));
                                success = false;
                            }
                            _ => unreachable!(),
                        }
                    }

                    if !success || !params_success || !returns_success {
                        return Err(());
                    }

                    let params = params
                        .into_iter()
                        .map(|p| {
                            if !p.name.is_empty() {
                                diagnostics.push(ast::Diagnostic::error(
                                    p.name_loc.unwrap(),
                                    "function type parameters cannot be named".to_string(),
                                ));
                                success = false;
                            }
                            p.ty
                        })
                        .collect();

                    let returns = returns
                        .into_iter()
                        .map(|p| {
                            if !p.name.is_empty() {
                                diagnostics.push(ast::Diagnostic::error(
                                    p.name_loc.unwrap(),
                                    "function type returns cannot be named".to_string(),
                                ));
                                success = false;
                            }
                            p.ty
                        })
                        .collect();

                    if is_external {
                        ast::Type::ExternalFunction {
                            params,
                            mutability,
                            returns,
                        }
                    } else {
                        ast::Type::InternalFunction {
                            params,
                            mutability,
                            returns,
                        }
                    }
                }
                pt::Type::Payable => {
                    if !casting {
                        diagnostics.push(ast::Diagnostic::decl_error(
                            id.loc(),
                            "‘payable’ cannot be used for type declarations, only casting. use ‘address payable’"
                                .to_string(),
                        ));
                        return Err(());
                    } else {
                        ast::Type::Address(true)
                    }
                }
                _ => ast::Type::from(ty),
            };

            return if dimensions.is_empty() {
                Ok(ty)
            } else {
                Ok(ast::Type::Array(
                    Box::new(ty),
                    resolve_dimensions(&dimensions, diagnostics)?,
                ))
            };
        }

        let id = match id {
            pt::Expression::Variable(id) => id,
            _ => unreachable!(),
        };

        let s = self.resolve_namespace(namespace, file_no, contract_no, &id, diagnostics)?;

        match s {
            None => {
                diagnostics.push(ast::Diagnostic::decl_error(
                    id.loc,
                    format!("type ‘{}’ not found", id.name),
                ));
                Err(())
            }
            Some(ast::Symbol::Enum(_, n)) if dimensions.is_empty() => Ok(ast::Type::Enum(*n)),
            Some(ast::Symbol::Enum(_, n)) => Ok(ast::Type::Array(
                Box::new(ast::Type::Enum(*n)),
                resolve_dimensions(&dimensions, diagnostics)?,
            )),
            Some(ast::Symbol::Struct(_, n)) if dimensions.is_empty() => Ok(ast::Type::Struct(*n)),
            Some(ast::Symbol::Struct(_, n)) => Ok(ast::Type::Array(
                Box::new(ast::Type::Struct(*n)),
                resolve_dimensions(&dimensions, diagnostics)?,
            )),
            Some(ast::Symbol::Contract(_, n)) if dimensions.is_empty() => {
                Ok(ast::Type::Contract(*n))
            }
            Some(ast::Symbol::Contract(_, n)) => Ok(ast::Type::Array(
                Box::new(ast::Type::Contract(*n)),
                resolve_dimensions(&dimensions, diagnostics)?,
            )),
            Some(ast::Symbol::Event(_)) => {
                diagnostics.push(ast::Diagnostic::decl_error(
                    id.loc,
                    format!("‘{}’ is an event", id.name),
                ));
                Err(())
            }
            Some(ast::Symbol::Function(_)) => {
                diagnostics.push(ast::Diagnostic::decl_error(
                    id.loc,
                    format!("‘{}’ is a function", id.name),
                ));
                Err(())
            }
            Some(ast::Symbol::Variable(_, _, _)) => {
                diagnostics.push(ast::Diagnostic::decl_error(
                    id.loc,
                    format!("‘{}’ is a contract variable", id.name),
                ));
                Err(())
            }
            Some(ast::Symbol::Import(_, _)) => {
                diagnostics.push(ast::Diagnostic::decl_error(
                    id.loc,
                    format!("‘{}’ is an import variable", id.name),
                ));
                Err(())
            }
        }
    }

    /// Resolve the type name with the namespace to a symbol
    fn resolve_namespace(
        &self,
        mut namespace: Vec<&pt::Identifier>,
        file_no: usize,
        mut contract_no: Option<usize>,
        id: &pt::Identifier,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Option<&ast::Symbol>, ()> {
        // The leading part of the namespace can be import variables
        let mut import_file_no = file_no;

        while !namespace.is_empty() {
            if let Some(ast::Symbol::Import(_, file_no)) =
                self.symbols
                    .get(&(import_file_no, None, namespace[0].name.clone()))
            {
                import_file_no = *file_no;
                namespace.remove(0);
                contract_no = None;
            } else {
                break;
            }
        }

        if let Some(contract_name) = namespace.get(0) {
            contract_no =
                match self
                    .symbols
                    .get(&(import_file_no, None, contract_name.name.clone()))
                {
                    None => {
                        diagnostics.push(ast::Diagnostic::decl_error(
                            id.loc,
                            format!("contract type ‘{}’ not found", id.name),
                        ));
                        return Err(());
                    }
                    Some(ast::Symbol::Contract(_, n)) => {
                        if namespace.len() > 1 {
                            diagnostics.push(ast::Diagnostic::decl_error(
                                id.loc,
                                format!("‘{}’ not found", namespace[1].name),
                            ));
                            return Err(());
                        };
                        Some(*n)
                    }
                    Some(ast::Symbol::Function(_)) => {
                        diagnostics.push(ast::Diagnostic::decl_error(
                            id.loc,
                            format!("‘{}’ is a function", id.name),
                        ));
                        return Err(());
                    }
                    Some(ast::Symbol::Variable(_, _, _)) => {
                        diagnostics.push(ast::Diagnostic::decl_error(
                            id.loc,
                            format!("‘{}’ is a contract variable", id.name),
                        ));
                        return Err(());
                    }
                    Some(ast::Symbol::Event(_)) => {
                        diagnostics.push(ast::Diagnostic::decl_error(
                            id.loc,
                            format!("‘{}’ is an event", id.name),
                        ));
                        return Err(());
                    }
                    Some(ast::Symbol::Struct(_, _)) => {
                        diagnostics.push(ast::Diagnostic::decl_error(
                            id.loc,
                            format!("‘{}’ is a struct", id.name),
                        ));
                        return Err(());
                    }
                    Some(ast::Symbol::Enum(_, _)) => {
                        diagnostics.push(ast::Diagnostic::decl_error(
                            id.loc,
                            format!("‘{}’ is an enum variable", id.name),
                        ));
                        return Err(());
                    }
                    Some(ast::Symbol::Import(_, _)) => unreachable!(),
                };
        }

        let mut s = self
            .symbols
            .get(&(import_file_no, contract_no, id.name.to_owned()));

        if let Some(contract_no) = contract_no {
            // check bases contracts
            if s.is_none() {
                if let Some(sym) = self.resolve_base_contract(contract_no, &id) {
                    s = Some(sym);
                }
            }

            // try global scope
            if s.is_none() {
                s = self
                    .symbols
                    .get(&(import_file_no, None, id.name.to_owned()));
            }
        }

        Ok(s)
    }

    // An array type can look like foo[2] foo.baz.bar, if foo is an enum type. The lalrpop parses
    // this as an expression, so we need to convert it to Type and check there are
    // no unexpected expressions types.
    #[allow(clippy::vec_init_then_push)]
    pub fn expr_to_type<'a>(
        &mut self,
        file_no: usize,
        contract_no: Option<usize>,
        expr: &'a pt::Expression,
        diagnostics: &mut Vec<ast::Diagnostic>,
    ) -> Result<(Vec<&'a pt::Identifier>, pt::Expression, Vec<ArrayDimension>), ()> {
        let mut expr = expr;
        let mut dimensions = vec![];

        loop {
            expr = match expr {
                pt::Expression::ArraySubscript(_, r, None) => {
                    dimensions.push(None);

                    r.as_ref()
                }
                pt::Expression::ArraySubscript(_, r, Some(index)) => {
                    dimensions.push(self.resolve_array_dimension(
                        file_no,
                        contract_no,
                        index,
                        diagnostics,
                    )?);

                    r.as_ref()
                }
                pt::Expression::Variable(_) | pt::Expression::Type(_, _) => {
                    return Ok((Vec::new(), expr.clone(), dimensions))
                }
                pt::Expression::MemberAccess(_, namespace, id) => {
                    let mut names = Vec::new();

                    let mut expr = namespace.as_ref();

                    while let pt::Expression::MemberAccess(_, member, name) = expr {
                        names.insert(0, name);

                        expr = member.as_ref();
                    }

                    if let pt::Expression::Variable(namespace) = expr {
                        names.insert(0, namespace);

                        return Ok((names, pt::Expression::Variable(id.clone()), dimensions));
                    } else {
                        diagnostics.push(ast::Diagnostic::decl_error(
                            namespace.loc(),
                            "expression found where type expected".to_string(),
                        ));
                        return Err(());
                    }
                }
                _ => {
                    diagnostics.push(ast::Diagnostic::decl_error(
                        expr.loc(),
                        "expression found where type expected".to_string(),
                    ));
                    return Err(());
                }
            }
        }
    }

    /// Resolve an expression which defines the array length, e.g. 2**8 in "bool[2**8]"
    fn resolve_array_dimension(
        &mut self,
        file_no: usize,
        contract_no: Option<usize>,
        expr: &pt::Expression,
        diagnostics: &mut Vec<ast::Diagnostic>,
    ) -> Result<ArrayDimension, ()> {
        let symtable = Symtable::new();

        let size_expr = expression(
            &expr,
            file_no,
            contract_no,
            self,
            &symtable,
            true,
            diagnostics,
            Some(&ast::Type::Uint(256)),
        )?;

        match size_expr.ty() {
            ast::Type::Uint(_) | ast::Type::Int(_) => {}
            _ => {
                diagnostics.push(ast::Diagnostic::decl_error(
                    expr.loc(),
                    "expression is not a number".to_string(),
                ));
                return Err(());
            }
        }

        match eval_const_number(&size_expr, contract_no, self) {
            Ok(n) => Ok(Some(n)),
            Err(d) => {
                diagnostics.push(d);

                Err(())
            }
        }
    }

    /// Phoney default constructor
    pub fn default_constructor(&self, contract_no: usize) -> ast::Function {
        let mut func = ast::Function::new(
            pt::Loc(0, 0, 0),
            "".to_owned(),
            Some(contract_no),
            vec![],
            pt::FunctionTy::Constructor,
            None,
            pt::Visibility::Public(pt::Loc(0, 0, 0)),
            Vec::new(),
            Vec::new(),
            self,
        );

        func.body = vec![ast::Statement::Return(pt::Loc(0, 0, 0), Vec::new())];
        func.has_body = true;

        func
    }

    /// Generate the signature for the given name and parameters. Can be used
    /// for both events and functions
    pub fn signature(&self, name: &str, params: &[ast::Parameter]) -> String {
        format!(
            "{}({})",
            name,
            params
                .iter()
                .map(|p| p.ty.to_signature_string(self))
                .collect::<Vec<String>>()
                .join(",")
        )
    }
}

impl ast::Symbol {
    /// Is this a private symbol
    pub fn is_private_variable(&self, ns: &ast::Namespace) -> bool {
        match self {
            ast::Symbol::Variable(_, Some(contract_no), var_no) => {
                let visibility = &ns.contracts[*contract_no].variables[*var_no].visibility;

                matches!(visibility, pt::Visibility::Private(_))
            }
            _ => false,
        }
    }
}
