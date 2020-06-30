use inkwell::OptimizationLevel;
use num_bigint::BigInt;
use num_traits::Signed;
use num_traits::Zero;
use output::{any_errors, Note, Output};
use parser::pt;
use std::collections::HashMap;
use Target;

mod address;
pub mod ast;
mod builtin;
pub mod eval;
pub mod expression;
mod functions;
mod mutability;
mod statements;
pub mod symtable;
mod types;
mod variables;

use codegen::cfg::ControlFlowGraph;
use emit;
use sema::ast::Statement;
use sema::eval::eval_const_number;
use sema::expression::expression;
use sema::symtable::Symtable;

pub type ArrayDimension = Option<(pt::Loc, BigInt)>;

pub fn sema(s: pt::SourceUnit, target: Target) -> ast::Namespace {
    // first resolve all the types we can find
    let mut ns = types::resolve(&s, target);

    // give up if we failed
    if any_errors(&ns.diagnostics) {
        return ns;
    }

    // we need to resolve declarations first, so we call functions/constructors of
    // contracts before they are declared
    let mut contract_no = 0;
    for part in &s.0 {
        if let pt::SourceUnitPart::ContractDefinition(def) = part {
            resolve_contract_declarations(def, contract_no, target, &mut ns);

            contract_no += 1;
        }
    }

    // Now we can resolve the bodies
    let mut contract_no = 0;
    for part in &s.0 {
        if let pt::SourceUnitPart::ContractDefinition(def) = part {
            resolve_contract_bodies(def, contract_no, &mut ns);

            contract_no += 1;
        }
    }

    // now check state mutability for all contracts
    mutability::mutablity(&mut ns);

    ns
}

/// Resolve functions declarations, constructor declarations, and contract variables
fn resolve_contract_declarations(
    def: &pt::ContractDefinition,
    contract_no: usize,
    target: Target,
    ns: &mut ast::Namespace,
) -> bool {
    ns.diagnostics.push(Output::info(
        def.loc,
        format!("found contract {}", def.name.name),
    ));

    let mut broken = false;

    // resolve function signatures
    for (i, parts) in def.parts.iter().enumerate() {
        if let pt::ContractPart::FunctionDefinition(ref f) = parts {
            if !functions::function_decl(f, i, contract_no, ns) {
                broken = true;
            }
        }
    }

    // resolve state variables
    if variables::contract_variables(&def, contract_no, ns) {
        broken = true;
    }

    // Substrate requires one constructor. Ideally we do not create implict things
    // in the ast, but this is required for abi generation which is done of the ast
    if !ns.contracts[contract_no]
        .functions
        .iter()
        .any(|f| f.is_constructor())
        && target == Target::Substrate
    {
        let mut fdecl = ast::Function::new(
            pt::Loc(0, 0),
            "".to_owned(),
            vec![],
            pt::FunctionTy::Constructor,
            None,
            None,
            pt::Visibility::Public(pt::Loc(0, 0)),
            Vec::new(),
            Vec::new(),
            ns,
        );

        fdecl.body = vec![Statement::Return(pt::Loc(0, 0), Vec::new())];

        ns.contracts[contract_no].functions.push(fdecl);
    }

    broken
}

fn resolve_contract_bodies(
    def: &pt::ContractDefinition,
    contract_no: usize,
    ns: &mut ast::Namespace,
) -> bool {
    let mut broken = false;

    // resolve function bodies
    for f in 0..ns.contracts[contract_no].functions.len() {
        if let Some(ast_index) = ns.contracts[contract_no].functions[f].ast_index {
            if let pt::ContractPart::FunctionDefinition(ref ast_f) = def.parts[ast_index] {
                if statements::resolve_function_body(ast_f, contract_no, f, ns).is_err() {
                    broken = true;
                }
            }
        }
    }

    broken
}

impl ast::Namespace {
    pub fn new(target: Target, address_length: usize) -> Self {
        ast::Namespace {
            target,
            enums: Vec::new(),
            structs: Vec::new(),
            contracts: Vec::new(),
            address_length,
            value_length: 16,
            symbols: HashMap::new(),
            diagnostics: Vec::new(),
        }
    }

    /// Add symbol to symbol table; either returns true for success, or adds an appropriate error
    pub fn add_symbol(
        &mut self,
        contract_no: Option<usize>,
        id: &pt::Identifier,
        symbol: ast::Symbol,
    ) -> bool {
        if builtin::is_reserved(&id.name) {
            self.diagnostics.push(Output::error(
                id.loc,
                format!("‘{}’ shadows name of a builtin", id.name.to_string()),
            ));

            return false;
        }

        if let Some(sym) = self.symbols.get(&(contract_no, id.name.to_owned())) {
            match sym {
                ast::Symbol::Contract(c, _) => {
                    self.diagnostics.push(Output::error_with_note(
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
                    self.diagnostics.push(Output::error_with_note(
                        id.loc,
                        format!("{} is already defined as an enum", id.name.to_string()),
                        *c,
                        "location of previous definition".to_string(),
                    ));
                }
                ast::Symbol::Struct(c, _) => {
                    self.diagnostics.push(Output::error_with_note(
                        id.loc,
                        format!("{} is already defined as a struct", id.name.to_string()),
                        *c,
                        "location of previous definition".to_string(),
                    ));
                }
                ast::Symbol::Variable(c, _) => {
                    self.diagnostics.push(Output::error_with_note(
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
                    self.diagnostics.push(Output::error_with_note(
                        id.loc,
                        format!("{} is already defined as a function", id.name.to_string()),
                        v[0].0,
                        "location of previous definition".to_string(),
                    ));
                }
            }

            return false;
        }

        // if there is nothing on the contract level, try top-level scope
        if contract_no.is_some() {
            if let Some(sym) = self.symbols.get(&(None, id.name.to_owned())) {
                match sym {
                    ast::Symbol::Contract(c, _) => {
                        self.diagnostics.push(Output::warning_with_note(
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
                        self.diagnostics.push(Output::warning_with_note(
                            id.loc,
                            format!("{} is already defined as an enum", id.name.to_string()),
                            *c,
                            "location of previous definition".to_string(),
                        ));
                    }
                    ast::Symbol::Struct(c, _) => {
                        self.diagnostics.push(Output::warning_with_note(
                            id.loc,
                            format!("{} is already defined as a struct", id.name.to_string()),
                            *c,
                            "location of previous definition".to_string(),
                        ));
                    }
                    ast::Symbol::Variable(c, _) => {
                        self.diagnostics.push(Output::warning_with_note(
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
                        self.diagnostics.push(Output::warning_with_note(
                            id.loc,
                            format!("{} is already defined as a function", id.name.to_string()),
                            v[0].0,
                            "location of previous definition".to_string(),
                        ));
                    }
                }
            }
        }

        self.symbols
            .insert((contract_no, id.name.to_string()), symbol);

        true
    }

    pub fn resolve_enum(&self, contract_no: Option<usize>, id: &pt::Identifier) -> Option<usize> {
        if let Some(ast::Symbol::Enum(_, n)) = self.symbols.get(&(contract_no, id.name.to_owned()))
        {
            return Some(*n);
        }

        if contract_no.is_some() {
            if let Some(ast::Symbol::Enum(_, n)) = self.symbols.get(&(None, id.name.to_owned())) {
                return Some(*n);
            }
        }

        None
    }

    pub fn resolve_contract(&self, id: &pt::Identifier) -> Option<usize> {
        if let Some(ast::Symbol::Contract(_, n)) = self.symbols.get(&(None, id.name.to_owned())) {
            return Some(*n);
        }

        None
    }

    pub fn resolve_func(
        &mut self,
        contract_no: usize,
        id: &pt::Identifier,
    ) -> Result<Vec<(pt::Loc, usize)>, ()> {
        match self.symbols.get(&(Some(contract_no), id.name.to_owned())) {
            Some(ast::Symbol::Function(v)) => Ok(v.clone()),
            _ => {
                self.diagnostics.push(Output::error(
                    id.loc,
                    "unknown function or type".to_string(),
                ));

                Err(())
            }
        }
    }

    pub fn resolve_var(&mut self, contract_no: usize, id: &pt::Identifier) -> Result<usize, ()> {
        let mut s = self.symbols.get(&(Some(contract_no), id.name.to_owned()));

        if s.is_none() {
            s = self.symbols.get(&(None, id.name.to_owned()));
        }

        match s {
            None => {
                self.diagnostics.push(Output::decl_error(
                    id.loc,
                    format!("`{}' is not declared", id.name),
                ));
                Err(())
            }
            Some(ast::Symbol::Enum(_, _)) => {
                self.diagnostics.push(Output::decl_error(
                    id.loc,
                    format!("`{}' is an enum", id.name),
                ));
                Err(())
            }
            Some(ast::Symbol::Struct(_, _)) => {
                self.diagnostics.push(Output::decl_error(
                    id.loc,
                    format!("`{}' is a struct", id.name),
                ));
                Err(())
            }
            Some(ast::Symbol::Function(_)) => {
                self.diagnostics.push(Output::decl_error(
                    id.loc,
                    format!("`{}' is a function", id.name),
                ));
                Err(())
            }
            Some(ast::Symbol::Contract(_, _)) => {
                self.diagnostics.push(Output::decl_error(
                    id.loc,
                    format!("`{}' is a contract", id.name),
                ));
                Err(())
            }
            Some(ast::Symbol::Variable(_, n)) => Ok(*n),
        }
    }

    pub fn check_shadowing(&mut self, contract_no: usize, id: &pt::Identifier) {
        let mut s = self.symbols.get(&(Some(contract_no), id.name.to_owned()));

        if s.is_none() {
            s = self.symbols.get(&(None, id.name.to_owned()));
        }

        match s {
            Some(ast::Symbol::Enum(loc, _)) => {
                self.diagnostics.push(Output::warning_with_note(
                    id.loc,
                    format!("declaration of `{}' shadows enum definition", id.name),
                    *loc,
                    "previous definition of enum".to_string(),
                ));
            }
            Some(ast::Symbol::Struct(loc, _)) => {
                self.diagnostics.push(Output::warning_with_note(
                    id.loc,
                    format!("declaration of `{}' shadows struct definition", id.name),
                    *loc,
                    "previous definition of struct".to_string(),
                ));
            }
            Some(ast::Symbol::Function(v)) => {
                let notes = v
                    .iter()
                    .map(|(pos, _)| Note {
                        pos: *pos,
                        message: "previous declaration of function".to_owned(),
                    })
                    .collect();
                self.diagnostics.push(Output::warning_with_notes(
                    id.loc,
                    format!("declaration of `{}' shadows function", id.name),
                    notes,
                ));
            }
            Some(ast::Symbol::Variable(loc, _)) => {
                self.diagnostics.push(Output::warning_with_note(
                    id.loc,
                    format!("declaration of `{}' shadows state variable", id.name),
                    *loc,
                    "previous declaration of state variable".to_string(),
                ));
            }
            Some(ast::Symbol::Contract(loc, _)) => {
                self.diagnostics.push(Output::warning_with_note(
                    id.loc,
                    format!("declaration of `{}' shadows contract name", id.name),
                    *loc,
                    "previous declaration of contract name".to_string(),
                ));
            }
            None => {}
        }
    }

    /// Resolve the parsed data type. The type can be a primitive, enum and also an arrays.
    /// The type for address payable is "address payble" used as a type, and "payable" when
    /// casting. So, we need to know what we are resolving for.
    pub fn resolve_type(
        &mut self,
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
                        ns.diagnostics.push(Output::decl_error(
                            *loc,
                            "zero size array not permitted".to_string(),
                        ));
                        return Err(());
                    } else if n.is_negative() {
                        ns.diagnostics.push(Output::decl_error(
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

        let (contract_name, id, dimensions) = self.expr_to_type(contract_no, &id)?;

        if let pt::Expression::Type(_, ty) = &id {
            assert_eq!(contract_name, None);

            let ty = match ty {
                pt::Type::Mapping(_, k, v) => {
                    let key = self.resolve_type(contract_no, false, k)?;
                    let value = self.resolve_type(contract_no, false, v)?;

                    match key {
                        ast::Type::Mapping(_, _) => {
                            self.diagnostics.push(Output::decl_error(
                                k.loc(),
                                "key of mapping cannot be another mapping type".to_string(),
                            ));
                            return Err(());
                        }
                        ast::Type::Struct(_) => {
                            self.diagnostics.push(Output::decl_error(
                                k.loc(),
                                "key of mapping cannot be struct type".to_string(),
                            ));
                            return Err(());
                        }
                        ast::Type::Array(_, _) => {
                            self.diagnostics.push(Output::decl_error(
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
                        self.diagnostics.push(Output::decl_error(
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

        let contract_no = if let Some(contract_name) = contract_name {
            match self.symbols.get(&(None, contract_name.name)) {
                None => {
                    self.diagnostics.push(Output::decl_error(
                        id.loc,
                        format!("contract type ‘{}’ not found", id.name),
                    ));
                    return Err(());
                }
                Some(ast::Symbol::Contract(_, n)) => Some(*n),
                Some(ast::Symbol::Function(_)) => {
                    self.diagnostics.push(Output::decl_error(
                        id.loc,
                        format!("‘{}’ is a function", id.name),
                    ));
                    return Err(());
                }
                Some(ast::Symbol::Variable(_, _)) => {
                    self.diagnostics.push(Output::decl_error(
                        id.loc,
                        format!("‘{}’ is a contract variable", id.name),
                    ));
                    return Err(());
                }
                Some(ast::Symbol::Struct(_, _)) => {
                    self.diagnostics.push(Output::decl_error(
                        id.loc,
                        format!("‘{}’ is a struct", id.name),
                    ));
                    return Err(());
                }
                Some(ast::Symbol::Enum(_, _)) => {
                    self.diagnostics.push(Output::decl_error(
                        id.loc,
                        format!("‘{}’ is an enum variable", id.name),
                    ));
                    return Err(());
                }
            }
        } else {
            contract_no
        };

        let mut s = self.symbols.get(&(contract_no, id.name.to_owned()));

        // try global scope
        if s.is_none() && contract_no.is_some() {
            s = self.symbols.get(&(None, id.name.to_owned()));
        }

        match s {
            None => {
                self.diagnostics.push(Output::decl_error(
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
                self.diagnostics.push(Output::decl_error(
                    id.loc,
                    format!("‘{}’ is a function", id.name),
                ));
                Err(())
            }
            Some(ast::Symbol::Variable(_, _)) => {
                self.diagnostics.push(Output::decl_error(
                    id.loc,
                    format!("‘{}’ is a contract variable", id.name),
                ));
                Err(())
            }
        }
    }

    // An array type can look like foo[2], if foo is an enum type. The lalrpop parses
    // this as an expression, so we need to convert it to Type and check there are
    // no unexpected expressions types.
    pub fn expr_to_type(
        &mut self,
        contract_no: Option<usize>,
        expr: &pt::Expression,
    ) -> Result<(Option<pt::Identifier>, pt::Expression, Vec<ArrayDimension>), ()> {
        let mut expr = expr;
        let mut dimensions = Vec::new();

        loop {
            expr = match expr {
                pt::Expression::ArraySubscript(_, r, None) => {
                    dimensions.push(None);

                    r.as_ref()
                }
                pt::Expression::ArraySubscript(_, r, Some(index)) => {
                    dimensions.push(self.resolve_array_dimension(contract_no, index)?);

                    r.as_ref()
                }
                pt::Expression::Variable(_) | pt::Expression::Type(_, _) => {
                    return Ok((None, expr.clone(), dimensions))
                }
                pt::Expression::MemberAccess(_, namespace, id) => {
                    if let pt::Expression::Variable(namespace) = namespace.as_ref() {
                        return Ok((
                            Some(namespace.clone()),
                            pt::Expression::Variable(id.clone()),
                            dimensions,
                        ));
                    } else {
                        self.diagnostics.push(Output::decl_error(
                            namespace.loc(),
                            "expression found where contract type expected".to_string(),
                        ));
                        return Err(());
                    }
                }
                _ => {
                    self.diagnostics.push(Output::decl_error(
                        expr.loc(),
                        "expression found where type expected".to_string(),
                    ));
                    return Err(());
                }
            }
        }
    }

    /// Resolve an expression which defines the array length, e.g. 2**8 in "bool[2**8]"
    pub fn resolve_array_dimension(
        &mut self,
        contract_no: Option<usize>,
        expr: &pt::Expression,
    ) -> Result<ArrayDimension, ()> {
        let symtable = Symtable::new();

        let size_expr = expression(&expr, contract_no, self, &symtable, true)?;
        match size_expr.ty() {
            ast::Type::Uint(_) | ast::Type::Int(_) => {}
            _ => {
                self.diagnostics.push(Output::decl_error(
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

impl ast::Contract {
    pub fn new(name: &str) -> Self {
        ast::Contract {
            name: name.to_owned(),
            doc: Vec::new(),
            functions: Vec::new(),
            variables: Vec::new(),
            top_of_contract_storage: BigInt::zero(),
            creates: Vec::new(),
            initializer: ControlFlowGraph::new(),
        }
    }

    /// Return the index of the fallback function, if any
    pub fn fallback_function(&self) -> Option<usize> {
        for (i, f) in self.functions.iter().enumerate() {
            if f.ty == pt::FunctionTy::Fallback {
                return Some(i);
            }
        }
        None
    }

    /// Return the index of the receive function, if any
    pub fn receive_function(&self) -> Option<usize> {
        for (i, f) in self.functions.iter().enumerate() {
            if f.ty == pt::FunctionTy::Receive {
                return Some(i);
            }
        }
        None
    }

    pub fn emit<'a>(
        &'a self,
        ns: &'a ast::Namespace,
        context: &'a inkwell::context::Context,
        filename: &'a str,
        opt: OptimizationLevel,
    ) -> emit::Contract {
        emit::Contract::build(context, self, ns, filename, opt)
    }

    /// Print the entire contract; storage initializers, constructors and functions and their CFGs
    pub fn print_to_string(&self, ns: &ast::Namespace) -> String {
        let mut out = format!("#\n# Contract: {}\n#\n\n", self.name);

        out += "# storage initializer\n";
        out += &self.initializer.to_string(self, ns);

        for func in self.functions.iter() {
            out += &format!("\n# {} {}\n", func.ty, func.signature);

            if let Some(ref cfg) = func.cfg {
                out += &cfg.to_string(self, ns);
            }
        }

        out
    }
}
