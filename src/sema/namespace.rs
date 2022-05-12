use super::{
    ast::{Diagnostic, Mutability, Namespace, Note, Parameter, RetrieveType, Symbol, Type},
    builtin,
    diagnostics::Diagnostics,
    eval::eval_const_number,
    expression::{expression, ExprContext, ResolveTo},
    resolve_params, resolve_returns,
    symtable::Symtable,
    visit_bases, ArrayDimension,
};
use crate::parser::{
    pt,
    pt::{CodeLocation, OptionalCodeLocation},
};
use crate::Target;
use num_bigint::BigInt;
use num_traits::Signed;
use num_traits::Zero;
use std::collections::HashMap;

impl Namespace {
    /// Create a namespace and populate with the parameters for the target
    pub fn new(target: Target) -> Self {
        let (address_length, value_length) = match target {
            Target::Ewasm => (20, 16),
            Target::Substrate {
                address_length,
                value_length,
            } => (address_length, value_length),
            Target::Solana => (32, 8),
        };

        let mut ns = Namespace {
            target,
            files: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            events: Vec::new(),
            using: Vec::new(),
            contracts: Vec::new(),
            user_types: Vec::new(),
            functions: Vec::new(),
            yul_functions: Vec::new(),
            constants: Vec::new(),
            address_length,
            value_length,
            variable_symbols: HashMap::new(),
            function_symbols: HashMap::new(),
            diagnostics: Diagnostics::default(),
            next_id: 0,
            var_constants: HashMap::new(),
            hover_overrides: HashMap::new(),
        };

        if target == Target::Solana {
            ns.add_solana_builtins();
        }

        ns
    }

    /// Add symbol to symbol table; either returns true for success, or adds an appropriate error
    pub fn add_symbol(
        &mut self,
        file_no: usize,
        contract_no: Option<usize>,
        id: &pt::Identifier,
        symbol: Symbol,
    ) -> bool {
        if builtin::is_reserved(&id.name) {
            self.diagnostics.push(Diagnostic::error(
                id.loc,
                format!("'{}' shadows name of a builtin", id.name),
            ));

            return false;
        }

        if let Some(Symbol::Function(v)) =
            self.function_symbols
                .get(&(file_no, contract_no, id.name.to_owned()))
        {
            let notes = v
                .iter()
                .map(|(pos, _)| Note {
                    pos: *pos,
                    message: "location of previous definition".to_owned(),
                })
                .collect();

            self.diagnostics.push(Diagnostic::error_with_notes(
                id.loc,
                format!("{} is already defined as a function", id.name),
                notes,
            ));

            return false;
        }

        if let Some(sym) = self
            .variable_symbols
            .get(&(file_no, contract_no, id.name.to_owned()))
        {
            match sym {
                Symbol::Contract(c, _) => {
                    self.diagnostics.push(Diagnostic::error_with_note(
                        id.loc,
                        format!("{} is already defined as a contract name", id.name),
                        *c,
                        "location of previous definition".to_string(),
                    ));
                }
                Symbol::Enum(c, _) => {
                    self.diagnostics.push(Diagnostic::error_with_note(
                        id.loc,
                        format!("{} is already defined as an enum", id.name),
                        *c,
                        "location of previous definition".to_string(),
                    ));
                }
                Symbol::Struct(c, _) => {
                    self.diagnostics.push(Diagnostic::error_with_note(
                        id.loc,
                        format!("{} is already defined as a struct", id.name),
                        *c,
                        "location of previous definition".to_string(),
                    ));
                }
                Symbol::Event(events) => {
                    self.diagnostics.push(Diagnostic::error_with_note(
                        id.loc,
                        format!("{} is already defined as an event", id.name),
                        events[0].0,
                        "location of previous definition".to_string(),
                    ));
                }
                Symbol::Variable(c, _, _) => {
                    self.diagnostics.push(Diagnostic::error_with_note(
                        id.loc,
                        format!("{} is already defined as a contract variable", id.name),
                        *c,
                        "location of previous definition".to_string(),
                    ));
                }
                Symbol::Import(loc, _) => {
                    self.diagnostics.push(Diagnostic::error_with_note(
                        id.loc,
                        format!("{} is already defined as an import", id.name),
                        *loc,
                        "location of previous definition".to_string(),
                    ));
                }
                Symbol::UserType(loc, _) => {
                    self.diagnostics.push(Diagnostic::error_with_note(
                        id.loc,
                        format!("{} is already defined as an user type", id.name),
                        *loc,
                        "location of previous definition".to_string(),
                    ));
                }
                Symbol::Function(_) => unreachable!(),
            }

            return false;
        }

        // if there is nothing on the contract level
        if contract_no.is_some() {
            if let Some(Symbol::Function(v)) =
                self.function_symbols
                    .get(&(file_no, None, id.name.to_owned()))
            {
                let notes = v
                    .iter()
                    .map(|(pos, _)| Note {
                        pos: *pos,
                        message: "location of previous definition".to_owned(),
                    })
                    .collect();

                self.diagnostics.push(Diagnostic::warning_with_notes(
                    id.loc,
                    format!("{} is already defined as a function", id.name),
                    notes,
                ));
            }

            if let Some(sym) = self
                .variable_symbols
                .get(&(file_no, None, id.name.to_owned()))
            {
                match sym {
                    Symbol::Contract(c, _) => {
                        self.diagnostics.push(Diagnostic::warning_with_note(
                            id.loc,
                            format!("{} is already defined as a contract name", id.name),
                            *c,
                            "location of previous definition".to_string(),
                        ));
                    }
                    Symbol::Enum(c, _) => {
                        self.diagnostics.push(Diagnostic::warning_with_note(
                            id.loc,
                            format!("{} is already defined as an enum", id.name),
                            *c,
                            "location of previous definition".to_string(),
                        ));
                    }
                    Symbol::Struct(c, _) => {
                        self.diagnostics.push(Diagnostic::warning_with_note(
                            id.loc,
                            format!("{} is already defined as a struct", id.name),
                            *c,
                            "location of previous definition".to_string(),
                        ));
                    }
                    Symbol::Event(_) if symbol.is_event() => (),
                    Symbol::Event(e) => {
                        self.diagnostics.push(Diagnostic::warning_with_note(
                            id.loc,
                            format!("{} is already defined as an event", id.name),
                            e[0].0,
                            "location of previous definition".to_string(),
                        ));
                    }
                    Symbol::Variable(c, _, _) => {
                        self.diagnostics.push(Diagnostic::warning_with_note(
                            id.loc,
                            format!("{} is already defined as a contract variable", id.name),
                            *c,
                            "location of previous definition".to_string(),
                        ));
                    }
                    Symbol::Function(_) => unreachable!(),
                    Symbol::Import(loc, _) => {
                        self.diagnostics.push(Diagnostic::warning_with_note(
                            id.loc,
                            format!("{} is already defined as an import", id.name),
                            *loc,
                            "location of previous definition".to_string(),
                        ));
                    }
                    Symbol::UserType(loc, _) => {
                        self.diagnostics.push(Diagnostic::warning_with_note(
                            id.loc,
                            format!("{} is already defined as an user type", id.name),
                            *loc,
                            "location of previous definition".to_string(),
                        ));
                    }
                }
            }
        }

        if let Symbol::Function(_) = &symbol {
            self.function_symbols
                .insert((file_no, contract_no, id.name.to_string()), symbol);
        } else {
            self.variable_symbols
                .insert((file_no, contract_no, id.name.to_string()), symbol);
        }

        true
    }

    /// Resolve enum by name
    pub fn resolve_enum(
        &self,
        file_no: usize,
        contract_no: Option<usize>,
        id: &pt::Identifier,
    ) -> Option<usize> {
        if let Some(Symbol::Enum(_, n)) =
            self.variable_symbols
                .get(&(file_no, contract_no, id.name.to_owned()))
        {
            return Some(*n);
        }

        if let Some(contract_no) = contract_no {
            if let Some(Symbol::Enum(_, n)) = self.resolve_var_base_contract(contract_no, id) {
                return Some(*n);
            }

            if let Some(Symbol::Enum(_, n)) =
                self.variable_symbols
                    .get(&(file_no, None, id.name.to_owned()))
            {
                return Some(*n);
            }
        }

        None
    }

    /// Resolve a contract name
    pub fn resolve_contract(&self, file_no: usize, id: &pt::Identifier) -> Option<usize> {
        if let Some(Symbol::Contract(_, n)) =
            self.variable_symbols
                .get(&(file_no, None, id.name.to_owned()))
        {
            return Some(*n);
        }

        None
    }

    /// Resolve a contract name with namespace
    pub fn resolve_contract_with_namespace(
        &mut self,
        file_no: usize,
        name: &pt::IdentifierPath,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<usize, ()> {
        let (id, namespace) = name
            .identifiers
            .split_last()
            .map(|(id, namespace)| (id, namespace.iter().collect()))
            .unwrap();

        let s = self.resolve_namespace(namespace, file_no, None, id, diagnostics)?;

        if let Some(Symbol::Contract(_, contract_no)) = s {
            Ok(*contract_no)
        } else {
            let error = Namespace::wrong_symbol(s, id);

            diagnostics.push(error);

            Err(())
        }
    }

    /// Resolve a free function name with namespace
    pub fn resolve_free_function_with_namespace(
        &mut self,
        file_no: usize,
        name: &pt::IdentifierPath,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Vec<(pt::Loc, usize)>, ()> {
        let (id, namespace) = name
            .identifiers
            .split_last()
            .map(|(id, namespace)| (id, namespace.iter().collect()))
            .unwrap();

        let s = self.resolve_namespace(namespace, file_no, None, id, diagnostics)?;

        if let Some(Symbol::Function(list)) = s {
            Ok(list.clone())
        } else {
            let error = Namespace::wrong_symbol(s, id);

            diagnostics.push(error);

            Err(())
        }
    }

    /// Resolve an event. We should only be resolving events for emit statements
    pub fn resolve_event(
        &mut self,
        file_no: usize,
        contract_no: Option<usize>,
        expr: &pt::Expression,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Vec<usize>, ()> {
        let (namespace, id, dimensions) =
            self.expr_to_type(file_no, contract_no, expr, diagnostics)?;

        if !dimensions.is_empty() {
            diagnostics.push(Diagnostic::decl_error(
                expr.loc(),
                "array type found where event type expected".to_string(),
            ));
            return Err(());
        }

        let id = match id {
            pt::Expression::Variable(id) => id,
            _ => {
                diagnostics.push(Diagnostic::decl_error(
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
                    match self.variable_symbols.get(&(
                        file_no,
                        Some(contract_no),
                        id.name.to_owned(),
                    )) {
                        None => (),
                        Some(Symbol::Event(ev)) => {
                            for (_, event_no) in ev {
                                events.push(*event_no);
                            }
                        }
                        sym => {
                            let error = Namespace::wrong_symbol(sym, &id);

                            diagnostics.push(error);

                            return Err(());
                        }
                    }

                    if let Some(sym) =
                        self.function_symbols
                            .get(&(file_no, Some(contract_no), id.name.to_owned()))
                    {
                        let error = Namespace::wrong_symbol(Some(sym), &id);

                        diagnostics.push(error);

                        return Err(());
                    }
                }
            }

            if let Some(sym) = self
                .function_symbols
                .get(&(file_no, None, id.name.to_owned()))
            {
                let error = Namespace::wrong_symbol(Some(sym), &id);

                diagnostics.push(error);

                return Err(());
            }

            return match self
                .variable_symbols
                .get(&(file_no, None, id.name.to_owned()))
            {
                None if events.is_empty() => {
                    diagnostics.push(Diagnostic::decl_error(
                        id.loc,
                        format!("event '{}' not found", id.name),
                    ));
                    Err(())
                }
                None => Ok(events),
                Some(Symbol::Event(ev)) => {
                    for (_, event_no) in ev {
                        events.push(*event_no);
                    }
                    Ok(events)
                }
                sym => {
                    let error = Namespace::wrong_symbol(sym, &id);

                    diagnostics.push(error);

                    Err(())
                }
            };
        }

        let s = self.resolve_namespace(namespace, file_no, contract_no, &id, diagnostics)?;

        if let Some(Symbol::Event(events)) = s {
            Ok(events.iter().map(|(_, event_no)| *event_no).collect())
        } else {
            let error = Namespace::wrong_symbol(s, &id);

            diagnostics.push(error);

            Err(())
        }
    }

    pub fn wrong_symbol(sym: Option<&Symbol>, id: &pt::Identifier) -> Diagnostic {
        match sym {
            None => Diagnostic::decl_error(id.loc, format!("'{}' not found", id.name)),
            Some(Symbol::Enum(..)) => {
                Diagnostic::decl_error(id.loc, format!("'{}' is an enum", id.name))
            }
            Some(Symbol::Struct(..)) => {
                Diagnostic::decl_error(id.loc, format!("'{}' is a struct", id.name))
            }
            Some(Symbol::Event(_)) => {
                Diagnostic::decl_error(id.loc, format!("'{}' is an event", id.name))
            }
            Some(Symbol::Function(_)) => {
                Diagnostic::decl_error(id.loc, format!("'{}' is a function", id.name))
            }
            Some(Symbol::Contract(..)) => {
                Diagnostic::decl_error(id.loc, format!("'{}' is a contract", id.name))
            }
            Some(Symbol::Import(..)) => {
                Diagnostic::decl_error(id.loc, format!("'{}' is an import", id.name))
            }
            Some(Symbol::UserType(..)) => {
                Diagnostic::decl_error(id.loc, format!("'{}' is an user type", id.name))
            }
            Some(Symbol::Variable(..)) => {
                Diagnostic::decl_error(id.loc, format!("'{}' is a contract variable", id.name))
            }
        }
    }

    /// Does a parent contract have a function symbol defined with this name (recursive)
    fn resolve_func_base_contract(
        &self,
        contract_no: usize,
        id: &pt::Identifier,
    ) -> Option<&Symbol> {
        for base in self.contracts[contract_no].bases.iter() {
            // find file this contract was defined in
            let file_no = self.contracts[base.contract_no].loc.file_no();

            let res = self
                .function_symbols
                .get(&(file_no, Some(base.contract_no), id.name.to_owned()))
                .or_else(|| self.resolve_func_base_contract(base.contract_no, id));

            if res.is_some() {
                return res;
            }
        }

        None
    }

    /// Does a parent contract have a non-func symbol defined with this name (recursive)
    fn resolve_var_base_contract(
        &self,
        contract_no: usize,
        id: &pt::Identifier,
    ) -> Option<&Symbol> {
        for base in self.contracts[contract_no].bases.iter() {
            // find file this contract was defined in
            let file_no = self.contracts[base.contract_no].loc.file_no();

            if let Some(sym) =
                self.variable_symbols
                    .get(&(file_no, Some(base.contract_no), id.name.to_owned()))
            {
                if let Symbol::Variable(_, var_contract_no, var_no) = sym {
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
                let res = self.resolve_var_base_contract(base.contract_no, id);

                if res.is_some() {
                    return res;
                }
            }
        }

        None
    }

    /// Resolve contract variable or function. Specify whether you wish to resolve
    /// a function or a variable; this will change the lookup order. A public contract
    /// will have both a accesssor function and variable, and the accessor function
    /// may show else where in the base contracts a function without body.
    pub fn resolve_var(
        &self,
        file_no: usize,
        contract_no: Option<usize>,
        id: &pt::Identifier,
        function_first: bool,
    ) -> Option<&Symbol> {
        let func = || {
            let mut s = self
                .function_symbols
                .get(&(file_no, contract_no, id.name.to_owned()));

            if s.is_none() {
                if let Some(contract_no) = contract_no {
                    s = self.resolve_func_base_contract(contract_no, id);
                }
            }

            s.or_else(|| {
                self.function_symbols
                    .get(&(file_no, None, id.name.to_owned()))
            })
        };

        let var = || {
            let mut s = self
                .variable_symbols
                .get(&(file_no, contract_no, id.name.to_owned()));

            if s.is_none() {
                if let Some(contract_no) = contract_no {
                    s = self.resolve_var_base_contract(contract_no, id);
                }
            }

            s.or_else(|| {
                self.variable_symbols
                    .get(&(file_no, None, id.name.to_owned()))
            })
        };

        if function_first {
            func().or_else(var)
        } else {
            var().or_else(func)
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
            self.diagnostics.push(Diagnostic::warning(
                id.loc,
                format!("'{}' shadows name of a builtin", id.name),
            ));
            return;
        }

        let s = self
            .variable_symbols
            .get(&(file_no, contract_no, id.name.to_owned()))
            .or_else(|| {
                self.function_symbols
                    .get(&(file_no, contract_no, id.name.to_owned()))
            })
            .or_else(|| {
                self.variable_symbols
                    .get(&(file_no, None, id.name.to_owned()))
            })
            .or_else(|| {
                self.function_symbols
                    .get(&(file_no, None, id.name.to_owned()))
            });

        match s {
            Some(Symbol::Enum(loc, _)) => {
                let loc = *loc;
                self.diagnostics.push(Diagnostic::warning_with_note(
                    id.loc,
                    format!("declaration of '{}' shadows enum definition", id.name),
                    loc,
                    "previous definition of enum".to_string(),
                ));
            }
            Some(Symbol::Struct(loc, _)) => {
                let loc = *loc;
                self.diagnostics.push(Diagnostic::warning_with_note(
                    id.loc,
                    format!("declaration of '{}' shadows struct definition", id.name),
                    loc,
                    "previous definition of struct".to_string(),
                ));
            }
            Some(Symbol::Event(events)) => {
                let notes = events
                    .iter()
                    .map(|(pos, _)| Note {
                        pos: *pos,
                        message: "previous definition of event".to_owned(),
                    })
                    .collect();

                self.diagnostics.push(Diagnostic::warning_with_notes(
                    id.loc,
                    format!("declaration of '{}' shadows event definition", id.name),
                    notes,
                ));
            }
            Some(Symbol::Function(v)) => {
                let notes = v
                    .iter()
                    .map(|(pos, _)| Note {
                        pos: *pos,
                        message: "previous declaration of function".to_owned(),
                    })
                    .collect();
                self.diagnostics.push(Diagnostic::warning_with_notes(
                    id.loc,
                    format!("declaration of '{}' shadows function", id.name),
                    notes,
                ));
            }
            Some(Symbol::Variable(loc, _, _)) => {
                let loc = *loc;
                self.diagnostics.push(Diagnostic::warning_with_note(
                    id.loc,
                    format!("declaration of '{}' shadows state variable", id.name),
                    loc,
                    "previous declaration of state variable".to_string(),
                ));
            }
            Some(Symbol::Contract(loc, _)) => {
                let loc = *loc;
                self.diagnostics.push(Diagnostic::warning_with_note(
                    id.loc,
                    format!("declaration of '{}' shadows contract name", id.name),
                    loc,
                    "previous declaration of contract name".to_string(),
                ));
            }
            Some(Symbol::UserType(loc, _)) => {
                let loc = *loc;
                self.diagnostics.push(Diagnostic::warning_with_note(
                    id.loc,
                    format!("declaration of '{}' shadows type", id.name),
                    loc,
                    "previous declaration of type".to_string(),
                ));
            }
            Some(Symbol::Import(loc, _)) => {
                let loc = *loc;
                self.diagnostics.push(Diagnostic::warning_with_note(
                    id.loc,
                    format!("declaration of '{}' shadows import", id.name),
                    loc,
                    "previous declaration of import".to_string(),
                ));
            }
            None => (),
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
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Type, ()> {
        fn resolve_dimensions(
            ast_dimensions: &[Option<(pt::Loc, BigInt)>],
            diagnostics: &mut Vec<Diagnostic>,
        ) -> Result<Vec<Option<BigInt>>, ()> {
            let mut dimensions = Vec::new();

            for d in ast_dimensions.iter().rev() {
                if let Some((loc, n)) = d {
                    if n.is_zero() {
                        diagnostics.push(Diagnostic::decl_error(
                            *loc,
                            "zero size array not permitted".to_string(),
                        ));
                        return Err(());
                    } else if n.is_negative() {
                        diagnostics.push(Diagnostic::decl_error(
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
            self.expr_to_type(file_no, contract_no, id, diagnostics)?;

        if let pt::Expression::Type(loc, ty) = &id {
            assert!(namespace.is_empty());

            let ty = match ty {
                pt::Type::Mapping(_, k, v) => {
                    let key = self.resolve_type(file_no, contract_no, false, k, diagnostics)?;
                    let value = self.resolve_type(file_no, contract_no, false, v, diagnostics)?;

                    match key {
                        Type::Mapping(..) => {
                            diagnostics.push(Diagnostic::decl_error(
                                k.loc(),
                                "key of mapping cannot be another mapping type".to_string(),
                            ));
                            return Err(());
                        }
                        Type::Struct(_) => {
                            diagnostics.push(Diagnostic::decl_error(
                                k.loc(),
                                "key of mapping cannot be struct type".to_string(),
                            ));
                            return Err(());
                        }
                        Type::Array(..) => {
                            diagnostics.push(Diagnostic::decl_error(
                                k.loc(),
                                "key of mapping cannot be array type".to_string(),
                            ));
                            return Err(());
                        }
                        _ => Type::Mapping(Box::new(key), Box::new(value)),
                    }
                }
                pt::Type::Function {
                    params,
                    attributes,
                    returns,
                } => {
                    let mut mutability: Option<pt::Mutability> = None;
                    let mut visibility: Option<pt::Visibility> = None;

                    let mut success = true;

                    for a in attributes {
                        match a {
                            pt::FunctionAttribute::Mutability(m) => {
                                if let Some(e) = &mutability {
                                    diagnostics.push(Diagnostic::error_with_note(
                                        m.loc(),
                                        format!("function type mutability redeclared '{}'", m),
                                        e.loc(),
                                        format!(
                                            "location of previous mutability declaration of '{}'",
                                            e
                                        ),
                                    ));
                                    success = false;
                                    continue;
                                }

                                if let pt::Mutability::Constant(loc) = m {
                                    diagnostics.push(Diagnostic::warning(
                                        *loc,
                                        "'constant' is deprecated. Use 'view' instead".to_string(),
                                    ));

                                    mutability = Some(pt::Mutability::View(*loc));
                                } else {
                                    mutability = Some(m.clone());
                                }
                            }
                            pt::FunctionAttribute::Visibility(v @ pt::Visibility::Internal(_))
                            | pt::FunctionAttribute::Visibility(v @ pt::Visibility::External(_))
                                if visibility.is_none() =>
                            {
                                visibility = Some(v.clone());
                            }
                            pt::FunctionAttribute::Visibility(v) => {
                                diagnostics.push(Diagnostic::error(
                                    v.loc().unwrap(),
                                    format!("function type cannot have visibility '{}'", v),
                                ));
                                success = false;
                            }
                            pt::FunctionAttribute::Immutable(loc) => {
                                diagnostics.push(Diagnostic::error(
                                    *loc,
                                    "function type cannot be 'immutable'".to_string(),
                                ));
                            }
                            _ => unreachable!(),
                        }
                    }

                    let is_external = match visibility {
                        None | Some(pt::Visibility::Internal(_)) => false,
                        Some(pt::Visibility::External(_)) => true,
                        Some(v) => {
                            diagnostics.push(Diagnostic::error(
                                v.loc().unwrap(),
                                format!("function type cannot have visibility attribute '{}'", v),
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

                    let (returns, trailing_attributes): (&[_], &[_]) = match &returns {
                        Some((returns, trailing_attributes)) => (returns, trailing_attributes),
                        None => (&[], &[]),
                    };

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
                            pt::FunctionAttribute::Immutable(loc) => {
                                diagnostics.push(Diagnostic::error(
                                    *loc,
                                    "function type cannot be 'immutable'".to_string(),
                                ));
                                success = false;
                            }
                            pt::FunctionAttribute::Mutability(m) => {
                                diagnostics.push(Diagnostic::error(
                                    m.loc(),
                                    format!("mutability '{}' cannot be declared after returns", m),
                                ));
                                success = false;
                            }
                            pt::FunctionAttribute::Visibility(v) => {
                                diagnostics.push(Diagnostic::error(
                                    v.loc().unwrap(),
                                    format!("function type cannot have visibility '{}'", v),
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
                            if let Some(name) = p.id {
                                diagnostics.push(Diagnostic::error(
                                    name.loc,
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
                            if let Some(name) = p.id {
                                diagnostics.push(Diagnostic::error(
                                    name.loc,
                                    "function type returns cannot be named".to_string(),
                                ));
                                success = false;
                            }
                            p.ty
                        })
                        .collect();

                    let mutability = match mutability {
                        None => Mutability::Nonpayable(*loc),
                        Some(pt::Mutability::Payable(loc)) => Mutability::Payable(loc),
                        Some(pt::Mutability::Pure(loc)) => Mutability::Pure(loc),
                        Some(pt::Mutability::View(loc)) => Mutability::View(loc),
                        Some(pt::Mutability::Constant(loc)) => Mutability::View(loc),
                    };

                    if is_external {
                        Type::ExternalFunction {
                            params,
                            mutability,
                            returns,
                        }
                    } else {
                        Type::InternalFunction {
                            params,
                            mutability,
                            returns,
                        }
                    }
                }
                pt::Type::Payable => {
                    if !casting {
                        diagnostics.push(Diagnostic::decl_error(
                            id.loc(),
                            "'payable' cannot be used for type declarations, only casting. use ‘address payable’"
                                .to_string(),
                        ));
                        return Err(());
                    } else {
                        Type::Address(true)
                    }
                }
                _ => Type::from(ty),
            };

            return if dimensions.is_empty() {
                Ok(ty)
            } else {
                Ok(Type::Array(
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
                diagnostics.push(Diagnostic::decl_error(
                    id.loc,
                    format!("type '{}' not found", id.name),
                ));
                Err(())
            }
            Some(Symbol::Enum(_, n)) if dimensions.is_empty() => Ok(Type::Enum(*n)),
            Some(Symbol::Enum(_, n)) => Ok(Type::Array(
                Box::new(Type::Enum(*n)),
                resolve_dimensions(&dimensions, diagnostics)?,
            )),
            Some(Symbol::Struct(_, n)) if dimensions.is_empty() => Ok(Type::Struct(*n)),
            Some(Symbol::Struct(_, n)) => Ok(Type::Array(
                Box::new(Type::Struct(*n)),
                resolve_dimensions(&dimensions, diagnostics)?,
            )),
            Some(Symbol::Contract(_, n)) if dimensions.is_empty() => Ok(Type::Contract(*n)),
            Some(Symbol::Contract(_, n)) => Ok(Type::Array(
                Box::new(Type::Contract(*n)),
                resolve_dimensions(&dimensions, diagnostics)?,
            )),
            Some(Symbol::Event(_)) => {
                diagnostics.push(Diagnostic::decl_error(
                    id.loc,
                    format!("'{}' is an event", id.name),
                ));
                Err(())
            }
            Some(Symbol::Function(_)) => {
                diagnostics.push(Diagnostic::decl_error(
                    id.loc,
                    format!("'{}' is a function", id.name),
                ));
                Err(())
            }
            Some(Symbol::Variable(..)) => {
                diagnostics.push(Diagnostic::decl_error(
                    id.loc,
                    format!("'{}' is a contract variable", id.name),
                ));
                Err(())
            }
            Some(Symbol::Import(..)) => {
                diagnostics.push(Diagnostic::decl_error(
                    id.loc,
                    format!("'{}' is an import variable", id.name),
                ));
                Err(())
            }
            Some(Symbol::UserType(_, n)) => Ok(Type::UserType(*n)),
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
    ) -> Result<Option<&Symbol>, ()> {
        // The leading part of the namespace can be import variables
        let mut import_file_no = file_no;

        while !namespace.is_empty() {
            if let Some(Symbol::Import(_, file_no)) =
                self.variable_symbols
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
            contract_no = match self
                .variable_symbols
                .get(&(import_file_no, None, contract_name.name.clone()))
                .or_else(|| {
                    self.function_symbols
                        .get(&(import_file_no, None, contract_name.name.clone()))
                }) {
                None => {
                    diagnostics.push(Diagnostic::decl_error(
                        contract_name.loc,
                        format!("'{}' not found", contract_name.name),
                    ));
                    return Err(());
                }
                Some(Symbol::Contract(_, n)) => {
                    if namespace.len() > 1 {
                        diagnostics.push(Diagnostic::decl_error(
                            id.loc,
                            format!("'{}' not found", namespace[1].name),
                        ));
                        return Err(());
                    };
                    Some(*n)
                }
                Some(Symbol::Function(_)) => {
                    diagnostics.push(Diagnostic::decl_error(
                        contract_name.loc,
                        format!("'{}' is a function", contract_name.name),
                    ));
                    return Err(());
                }
                Some(Symbol::Variable(..)) => {
                    diagnostics.push(Diagnostic::decl_error(
                        contract_name.loc,
                        format!("'{}' is a contract variable", contract_name.name),
                    ));
                    return Err(());
                }
                Some(Symbol::Event(_)) => {
                    diagnostics.push(Diagnostic::decl_error(
                        contract_name.loc,
                        format!("'{}' is an event", contract_name.name),
                    ));
                    return Err(());
                }
                Some(Symbol::Struct(..)) => {
                    diagnostics.push(Diagnostic::decl_error(
                        contract_name.loc,
                        format!("'{}' is a struct", contract_name.name),
                    ));
                    return Err(());
                }
                Some(Symbol::Enum(..)) => {
                    diagnostics.push(Diagnostic::decl_error(
                        contract_name.loc,
                        format!("'{}' is an enum variable", contract_name.name),
                    ));
                    return Err(());
                }
                Some(Symbol::UserType(..)) => {
                    diagnostics.push(Diagnostic::decl_error(
                        contract_name.loc,
                        format!("'{}' is an user type", contract_name.name),
                    ));
                    return Err(());
                }
                Some(Symbol::Import(..)) => unreachable!(),
            };
        }

        let mut s = self
            .variable_symbols
            .get(&(import_file_no, contract_no, id.name.to_owned()))
            .or_else(|| {
                self.function_symbols
                    .get(&(import_file_no, contract_no, id.name.to_owned()))
            });

        if let Some(contract_no) = contract_no {
            // check bases contracts
            if s.is_none() {
                if let Some(sym) = self.resolve_var_base_contract(contract_no, id) {
                    s = Some(sym);
                }
            }

            // try global scope
            if s.is_none() {
                s = self
                    .variable_symbols
                    .get(&(import_file_no, None, id.name.to_owned()));
            }

            if s.is_none() {
                s = self
                    .function_symbols
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
        diagnostics: &mut Vec<Diagnostic>,
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
                        None,
                        index,
                        diagnostics,
                    )?);

                    r.as_ref()
                }
                pt::Expression::Variable(_) | pt::Expression::Type(..) => {
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
                        diagnostics.push(Diagnostic::decl_error(
                            namespace.loc(),
                            "expression found where type expected".to_string(),
                        ));
                        return Err(());
                    }
                }
                _ => {
                    diagnostics.push(Diagnostic::decl_error(
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
        function_no: Option<usize>,
        expr: &pt::Expression,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<ArrayDimension, ()> {
        let mut symtable = Symtable::new();
        let context = ExprContext {
            file_no,
            unchecked: true,
            contract_no,
            function_no,
            constant: true,
            lvalue: false,
            yul_function: false,
        };

        let size_expr = expression(
            expr,
            &context,
            self,
            &mut symtable,
            diagnostics,
            ResolveTo::Type(&Type::Uint(256)),
        )?;

        match size_expr.ty() {
            Type::Uint(_) | Type::Int(_) => {}
            _ => {
                diagnostics.push(Diagnostic::decl_error(
                    expr.loc(),
                    "expression is not a number".to_string(),
                ));
                return Err(());
            }
        }

        match eval_const_number(&size_expr, self) {
            Ok(n) => Ok(Some(n)),
            Err(d) => {
                diagnostics.push(d);

                Err(())
            }
        }
    }

    /// Generate the signature for the given name and parameters. Can be used
    /// for both events and functions
    pub fn signature(&self, name: &str, params: &[Parameter]) -> String {
        format!(
            "{}({})",
            name,
            params
                .iter()
                .map(|p| p.ty.to_signature_string(false, self))
                .collect::<Vec<String>>()
                .join(",")
        )
    }
}
