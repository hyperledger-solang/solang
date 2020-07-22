use num_bigint::BigInt;
use num_traits::Signed;
use num_traits::Zero;
use parser::{parse, pt};
use std::collections::HashMap;
use Target;

mod address;
pub mod ast;
mod builtin;
mod contracts;
pub mod diagnostics;
pub mod eval;
pub mod expression;
mod functions;
mod mutability;
mod statements;
pub mod symtable;
mod types;
mod variables;

use self::eval::eval_const_number;
use self::expression::expression;
use self::symtable::Symtable;
use file_cache::FileCache;

pub type ArrayDimension = Option<(pt::Loc, BigInt)>;

/// Load a file file from the cache, parse and resolve it. The file must be present in
/// the cache. This function is recursive for imports.
pub fn sema(filename: &str, cache: &mut FileCache, ns: &mut ast::Namespace) {
    let file_no = ns.files.len();

    ns.files.push(filename.to_string());

    let source_code = cache.get_file_contents(filename);

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
    let structs_to_resolve = types::resolve_typenames(&pt, file_no, ns);

    // resolve pragmas and imports
    for part in &pt.0 {
        match part {
            pt::SourceUnitPart::PragmaDirective(name, value) => {
                resolve_pragma(name, value, ns);
            }
            pt::SourceUnitPart::ImportDirective(import) => {
                resolve_import(import, file_no, cache, ns);
            }
            _ => (),
        }
    }

    // once all the types are resolved, we can resolve the structs. This is because struct
    // fields can have types defined elsewhere.
    types::resolve_structs(structs_to_resolve, file_no, ns);

    // give up if we failed
    if diagnostics::any_errors(&ns.diagnostics) {
        return;
    }

    // now resolve the contracts
    contracts::resolve(&contracts_to_resolve, file_no, ns);

    // now check state mutability for all contracts
    mutability::mutablity(file_no, ns);
}

/// Find import file, resolve it by calling sema and add it to the namespace
fn resolve_import(
    import: &pt::Import,
    file_no: usize,
    cache: &mut FileCache,
    ns: &mut ast::Namespace,
) {
    let filename = match import {
        pt::Import::Plain(f) => f,
        pt::Import::GlobalSymbol(f, _) => f,
        pt::Import::Rename(f, _) => f,
    };

    // We may already have resolved it
    if !ns.files.contains(&filename.string) {
        if let Err(message) = cache.populate_cache(&filename.string) {
            ns.diagnostics
                .push(ast::Diagnostic::error(filename.loc, message));

            return;
        }

        sema(&filename.string, cache, ns);

        // give up if we failed
        if diagnostics::any_errors(&ns.diagnostics) {
            return;
        }
    }

    let import_file_no = ns
        .files
        .iter()
        .position(|f| f == &filename.string)
        .expect("import should be loaded by now");

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
                if let Some(existing) = ns.symbols.get(&(file_no, None, name.clone())) {
                    if existing == &symbol {
                        continue;
                    }
                }

                ns.check_shadowing(file_no, None, &new_symbol);

                ns.add_symbol(file_no, None, &new_symbol, symbol);
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
        ns.diagnostics.push(ast::Diagnostic::info(
            pt::Loc(name.loc.0, name.loc.1, value.loc.2),
            "pragma ‘solidity’ is ignored".to_string(),
        ));
    } else if name.name == "experimental" && value.string == "ABIEncoderV2" {
        ns.diagnostics.push(ast::Diagnostic::info(
            pt::Loc(name.loc.0, name.loc.1, value.loc.2),
            "pragma ‘experimental’ with value ‘ABIEncoderV2’ is ignored".to_string(),
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
            contracts: Vec::new(),
            address_length,
            value_length,
            symbols: HashMap::new(),
            diagnostics: Vec::new(),
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
                ast::Symbol::Variable(c, _) => {
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
                    self.diagnostics.push(ast::Diagnostic::error_with_note(
                        id.loc,
                        format!("{} is already defined as a function", id.name.to_string()),
                        v[0].0,
                        "location of previous definition".to_string(),
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

        // if there is nothing on the contract level, try top-level scope
        if contract_no.is_some() {
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
                            format!("{} is already defined as an enum", id.name.to_string()),
                            *c,
                            "location of previous definition".to_string(),
                        ));
                    }
                    ast::Symbol::Struct(c, _) => {
                        self.diagnostics.push(ast::Diagnostic::warning_with_note(
                            id.loc,
                            format!("{} is already defined as a struct", id.name.to_string()),
                            *c,
                            "location of previous definition".to_string(),
                        ));
                    }
                    ast::Symbol::Variable(c, _) => {
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
                        self.diagnostics.push(ast::Diagnostic::warning_with_note(
                            id.loc,
                            format!("{} is already defined as a function", id.name.to_string()),
                            v[0].0,
                            "location of previous definition".to_string(),
                        ));
                    }
                    ast::Symbol::Import(loc, _) => {
                        self.diagnostics.push(ast::Diagnostic::warning_with_note(
                            id.loc,
                            format!("{} is already defined as an import", id.name.to_string()),
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

        if contract_no.is_some() {
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

    /// Resolve function name
    pub fn resolve_func(
        &mut self,
        file_no: usize,
        contract_no: usize,
        id: &pt::Identifier,
    ) -> Result<Vec<(pt::Loc, usize)>, ()> {
        match self
            .symbols
            .get(&(file_no, Some(contract_no), id.name.to_owned()))
        {
            Some(ast::Symbol::Function(v)) => Ok(v.clone()),
            _ => {
                self.diagnostics.push(ast::Diagnostic::error(
                    id.loc,
                    format!("unknown function or type ‘{}’", id.name),
                ));

                Err(())
            }
        }
    }

    /// Does a parent contract have a variable defined with this name (recursive)
    fn resolve_var_parent_contract(
        &self,
        contract_no: usize,
        id: &pt::Identifier,
    ) -> Option<(usize, usize)> {
        for contract_no in self.contracts[contract_no].inherit.iter() {
            // find file this contract was defined in
            let file_no = self.contracts[*contract_no].loc.0;

            if let Some(ast::Symbol::Variable(_, var_no)) =
                self.symbols
                    .get(&(file_no, Some(*contract_no), id.name.to_owned()))
            {
                let var = &self.contracts[*contract_no].variables[*var_no];

                if let pt::Visibility::Private(_) = var.visibility {
                    // not visible
                } else {
                    return Some((*contract_no, *var_no));
                }
            } else {
                let res = self.resolve_var_parent_contract(*contract_no, id);

                if res.is_some() {
                    return res;
                }
            }
        }

        None
    }

    /// Resolve contract variable
    pub fn resolve_var(
        &mut self,
        file_no: usize,
        contract_no: usize,
        id: &pt::Identifier,
    ) -> Result<(usize, usize), ()> {
        let mut s = self
            .symbols
            .get(&(file_no, Some(contract_no), id.name.to_owned()));

        if s.is_none() {
            if let Some((contract_no, var_no)) = self.resolve_var_parent_contract(contract_no, id) {
                return Ok((contract_no, var_no));
            }
        }

        if s.is_none() {
            s = self.symbols.get(&(file_no, None, id.name.to_owned()));
        }

        match s {
            None => {
                self.diagnostics.push(ast::Diagnostic::decl_error(
                    id.loc,
                    format!("`{}' is not declared", id.name),
                ));
                Err(())
            }
            Some(ast::Symbol::Enum(_, _)) => {
                self.diagnostics.push(ast::Diagnostic::decl_error(
                    id.loc,
                    format!("`{}' is an enum", id.name),
                ));
                Err(())
            }
            Some(ast::Symbol::Struct(_, _)) => {
                self.diagnostics.push(ast::Diagnostic::decl_error(
                    id.loc,
                    format!("`{}' is a struct", id.name),
                ));
                Err(())
            }
            Some(ast::Symbol::Function(_)) => {
                self.diagnostics.push(ast::Diagnostic::decl_error(
                    id.loc,
                    format!("`{}' is a function", id.name),
                ));
                Err(())
            }
            Some(ast::Symbol::Contract(_, _)) => {
                self.diagnostics.push(ast::Diagnostic::decl_error(
                    id.loc,
                    format!("`{}' is a contract", id.name),
                ));
                Err(())
            }
            Some(ast::Symbol::Import(_, _)) => {
                self.diagnostics.push(ast::Diagnostic::decl_error(
                    id.loc,
                    format!("`{}' is an import", id.name),
                ));
                Err(())
            }
            Some(ast::Symbol::Variable(_, n)) => Ok((contract_no, *n)),
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
            Some(ast::Symbol::Variable(loc, _)) => {
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
    ) -> Result<ast::Type, ()> {
        fn resolve_dimensions(
            ast_dimensions: &[Option<(pt::Loc, BigInt)>],
            ns: &mut ast::Namespace,
        ) -> Result<Vec<Option<BigInt>>, ()> {
            let mut dimensions = Vec::new();

            for d in ast_dimensions.iter().rev() {
                if let Some((loc, n)) = d {
                    if n.is_zero() {
                        ns.diagnostics.push(ast::Diagnostic::decl_error(
                            *loc,
                            "zero size array not permitted".to_string(),
                        ));
                        return Err(());
                    } else if n.is_negative() {
                        ns.diagnostics.push(ast::Diagnostic::decl_error(
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

        let (mut namespace, id, dimensions) = self.expr_to_type(file_no, contract_no, &id)?;

        if let pt::Expression::Type(_, ty) = &id {
            assert!(namespace.is_empty());

            let ty = match ty {
                pt::Type::Mapping(_, k, v) => {
                    let key = self.resolve_type(file_no, contract_no, false, k)?;
                    let value = self.resolve_type(file_no, contract_no, false, v)?;

                    match key {
                        ast::Type::Mapping(_, _) => {
                            self.diagnostics.push(ast::Diagnostic::decl_error(
                                k.loc(),
                                "key of mapping cannot be another mapping type".to_string(),
                            ));
                            return Err(());
                        }
                        ast::Type::Struct(_) => {
                            self.diagnostics.push(ast::Diagnostic::decl_error(
                                k.loc(),
                                "key of mapping cannot be struct type".to_string(),
                            ));
                            return Err(());
                        }
                        ast::Type::Array(_, _) => {
                            self.diagnostics.push(ast::Diagnostic::decl_error(
                                k.loc(),
                                "key of mapping cannot be array type".to_string(),
                            ));
                            return Err(());
                        }
                        _ => ast::Type::Mapping(Box::new(key), Box::new(value)),
                    }
                }
                pt::Type::Payable => {
                    if !casting {
                        self.diagnostics.push(ast::Diagnostic::decl_error(
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
                    resolve_dimensions(&dimensions, self)?,
                ))
            };
        }

        let id = match id {
            pt::Expression::Variable(id) => id,
            _ => unreachable!(),
        };

        // The leading part of the namespace can be import variables
        let mut import_file_no = file_no;

        while !namespace.is_empty() {
            if let Some(ast::Symbol::Import(_, file_no)) =
                self.symbols
                    .get(&(import_file_no, None, namespace[0].name.clone()))
            {
                import_file_no = *file_no;
                namespace.remove(0);
            } else {
                break;
            }
        }

        let contract_no = if let Some(contract_name) = namespace.get(0) {
            let contract_no =
                match self
                    .symbols
                    .get(&(import_file_no, None, contract_name.name.clone()))
                {
                    None => {
                        self.diagnostics.push(ast::Diagnostic::decl_error(
                            id.loc,
                            format!("contract type ‘{}’ not found", id.name),
                        ));
                        return Err(());
                    }
                    Some(ast::Symbol::Contract(_, n)) => Some(*n),
                    Some(ast::Symbol::Function(_)) => {
                        self.diagnostics.push(ast::Diagnostic::decl_error(
                            id.loc,
                            format!("‘{}’ is a function", id.name),
                        ));
                        return Err(());
                    }
                    Some(ast::Symbol::Variable(_, _)) => {
                        self.diagnostics.push(ast::Diagnostic::decl_error(
                            id.loc,
                            format!("‘{}’ is a contract variable", id.name),
                        ));
                        return Err(());
                    }
                    Some(ast::Symbol::Struct(_, _)) => {
                        self.diagnostics.push(ast::Diagnostic::decl_error(
                            id.loc,
                            format!("‘{}’ is a struct", id.name),
                        ));
                        return Err(());
                    }
                    Some(ast::Symbol::Enum(_, _)) => {
                        self.diagnostics.push(ast::Diagnostic::decl_error(
                            id.loc,
                            format!("‘{}’ is an enum variable", id.name),
                        ));
                        return Err(());
                    }
                    Some(ast::Symbol::Import(_, _)) => unreachable!(),
                };

            if namespace.len() > 1 {
                self.diagnostics.push(ast::Diagnostic::decl_error(
                    id.loc,
                    format!("‘{}’ not found", namespace[1].name),
                ));
                return Err(());
            };

            contract_no
        } else {
            contract_no
        };

        let mut s = self
            .symbols
            .get(&(import_file_no, contract_no, id.name.to_owned()));

        // try global scope
        if s.is_none() && contract_no.is_some() {
            s = self
                .symbols
                .get(&(import_file_no, None, id.name.to_owned()));
        }

        match s {
            None => {
                self.diagnostics.push(ast::Diagnostic::decl_error(
                    id.loc,
                    format!("type ‘{}’ not found", id.name),
                ));
                Err(())
            }
            Some(ast::Symbol::Enum(_, n)) if dimensions.is_empty() => Ok(ast::Type::Enum(*n)),
            Some(ast::Symbol::Enum(_, n)) => Ok(ast::Type::Array(
                Box::new(ast::Type::Enum(*n)),
                resolve_dimensions(&dimensions, self)?,
            )),
            Some(ast::Symbol::Struct(_, n)) if dimensions.is_empty() => Ok(ast::Type::Struct(*n)),
            Some(ast::Symbol::Struct(_, n)) => Ok(ast::Type::Array(
                Box::new(ast::Type::Struct(*n)),
                resolve_dimensions(&dimensions, self)?,
            )),
            Some(ast::Symbol::Contract(_, n)) if dimensions.is_empty() => {
                Ok(ast::Type::Contract(*n))
            }
            Some(ast::Symbol::Contract(_, n)) => Ok(ast::Type::Array(
                Box::new(ast::Type::Contract(*n)),
                resolve_dimensions(&dimensions, self)?,
            )),
            Some(ast::Symbol::Function(_)) => {
                self.diagnostics.push(ast::Diagnostic::decl_error(
                    id.loc,
                    format!("‘{}’ is a function", id.name),
                ));
                Err(())
            }
            Some(ast::Symbol::Variable(_, _)) => {
                self.diagnostics.push(ast::Diagnostic::decl_error(
                    id.loc,
                    format!("‘{}’ is a contract variable", id.name),
                ));
                Err(())
            }
            Some(ast::Symbol::Import(_, _)) => {
                self.diagnostics.push(ast::Diagnostic::decl_error(
                    id.loc,
                    format!("‘{}’ is an import variable", id.name),
                ));
                Err(())
            }
        }
    }

    // An array type can look like foo[2] foo.baz.bar, if foo is an enum type. The lalrpop parses
    // this as an expression, so we need to convert it to Type and check there are
    // no unexpected expressions types.
    pub fn expr_to_type<'a>(
        &mut self,
        file_no: usize,
        contract_no: Option<usize>,
        expr: &'a pt::Expression,
    ) -> Result<(Vec<&'a pt::Identifier>, pt::Expression, Vec<ArrayDimension>), ()> {
        let mut expr = expr;
        let mut dimensions = Vec::new();

        loop {
            expr = match expr {
                pt::Expression::ArraySubscript(_, r, None) => {
                    dimensions.push(None);

                    r.as_ref()
                }
                pt::Expression::ArraySubscript(_, r, Some(index)) => {
                    dimensions.push(self.resolve_array_dimension(file_no, contract_no, index)?);

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
                        self.diagnostics.push(ast::Diagnostic::decl_error(
                            namespace.loc(),
                            "expression found where type expected".to_string(),
                        ));
                        return Err(());
                    }
                }
                _ => {
                    self.diagnostics.push(ast::Diagnostic::decl_error(
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
    ) -> Result<ArrayDimension, ()> {
        let symtable = Symtable::new();

        let size_expr = expression(&expr, file_no, contract_no, self, &symtable, true)?;

        match size_expr.ty() {
            ast::Type::Uint(_) | ast::Type::Int(_) => {}
            _ => {
                self.diagnostics.push(ast::Diagnostic::decl_error(
                    expr.loc(),
                    "expression is not a number".to_string(),
                ));
                return Err(());
            }
        }

        match eval_const_number(&size_expr, contract_no, self) {
            Ok(n) => Ok(Some(n)),
            Err(d) => {
                self.diagnostics.push(d);

                Err(())
            }
        }
    }
}
