use inkwell::OptimizationLevel;
use num_bigint::BigInt;
use num_traits::Zero;
use parser::pt;
use std::collections::HashMap;
use std::collections::HashSet;
use std::iter::FromIterator;
use Target;

use super::ast;
use super::expression::{expression, match_constructor_to_args};
use super::functions;
use super::statements;
use super::symtable::Symtable;
use super::variables;
use codegen::cfg::ControlFlowGraph;
use emit;

impl ast::Contract {
    /// Create a new contract, abstract contract, interface or library
    pub fn new(name: &str, ty: pt::ContractTy, loc: pt::Loc) -> Self {
        ast::Contract {
            name: name.to_owned(),
            loc,
            ty,
            bases: Vec::new(),
            layout: Vec::new(),
            doc: Vec::new(),
            functions: Vec::new(),
            function_table: HashMap::new(),
            variables: Vec::new(),
            creates: Vec::new(),
            initializer: ControlFlowGraph::new(),
        }
    }

    /// Generate contract code for this contract
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

        for (contract_no, function_no, cfg) in self.function_table.values() {
            let func = &ns.contracts[*contract_no].functions[*function_no];
            let contract_name = &ns.contracts[*contract_no].name;

            out += &format!("\n# {} {}.{}\n", func.ty, contract_name, func.signature);

            out += &cfg.as_ref().unwrap().to_string(self, ns);
        }

        out
    }
}

/// Resolve the following contract
pub fn resolve(
    contracts: &[(usize, &pt::ContractDefinition)],
    file_no: usize,
    ns: &mut ast::Namespace,
) {
    resolve_base_contracts(contracts, file_no, ns);

    // we need to resolve declarations first, so we call functions/constructors of
    // contracts before they are declared
    let mut function_bodies = Vec::new();

    for (contract_no, def) in contracts {
        function_bodies.extend(resolve_declarations(def, file_no, *contract_no, ns));
    }

    // Resolve base contract constructor arguments on contract definition (not constructor definitions)
    resolve_base_args(contracts, file_no, ns);

    for (contract_no, _) in contracts {
        check_base_args(*contract_no, ns);
    }

    // Now we have all the declarations, we can create the layout of storage and handle base contracts
    for (contract_no, _) in contracts {
        layout_contract(*contract_no, ns);
    }

    // Now we can resolve the bodies
    resolve_bodies(function_bodies, file_no, ns);
}

/// Resolve the base contracts list and check for cycles. Returns true if no
/// issues where found.
fn resolve_base_contracts(
    contracts: &[(usize, &pt::ContractDefinition)],
    file_no: usize,
    ns: &mut ast::Namespace,
) {
    for (contract_no, def) in contracts {
        for base in &def.base {
            let name = &base.name;
            match ns.resolve_contract(file_no, name) {
                Some(no) => {
                    if no == *contract_no {
                        ns.diagnostics.push(ast::Diagnostic::error(
                            name.loc,
                            format!(
                                "contract ‘{}’ cannot have itself as a base contract",
                                name.name
                            ),
                        ));
                    } else if ns.contracts[*contract_no]
                        .bases
                        .iter()
                        .any(|e| e.contract_no == no)
                    {
                        ns.diagnostics.push(ast::Diagnostic::error(
                            name.loc,
                            format!(
                                "contract ‘{}’ duplicate base ‘{}’",
                                ns.contracts[*contract_no].name, name.name
                            ),
                        ));
                    } else if is_base(*contract_no, no, ns) {
                        ns.diagnostics.push(ast::Diagnostic::error(
                            name.loc,
                            format!(
                                "base ‘{}’ from contract ‘{}’ is cyclic",
                                name.name, ns.contracts[*contract_no].name
                            ),
                        ));
                    } else {
                        // We do not resolve the constructor arguments here, since we have not
                        // resolved any variables. This means no constants can be used on base
                        // constructor args, so we delay this until resolve_base_args()
                        ns.contracts[*contract_no].bases.push(ast::Base {
                            contract_no: no,
                            constructor: None,
                        });
                    }
                }
                None => {
                    ns.diagnostics.push(ast::Diagnostic::error(
                        name.loc,
                        format!("contract ‘{}’ not found", name.name),
                    ));
                }
            }
        }
    }
}

/// Resolve the base contracts list and check for cycles. Returns true if no
/// issues where found.
fn resolve_base_args(
    contracts: &[(usize, &pt::ContractDefinition)],
    file_no: usize,
    ns: &mut ast::Namespace,
) {
    // for every contract, if we have a base which resolved successfully, resolve any constructor args
    for (contract_no, def) in contracts {
        for base in &def.base {
            let name = &base.name;
            if let Some(base_no) = ns.resolve_contract(file_no, name) {
                if let Some(pos) = ns.contracts[*contract_no]
                    .bases
                    .iter()
                    .position(|e| e.contract_no == base_no)
                {
                    if let Some(args) = &base.args {
                        let mut resolved_args = Vec::new();
                        let symtable = Symtable::new();

                        for arg in args {
                            if let Ok(e) =
                                expression(&arg, file_no, Some(*contract_no), ns, &symtable, true)
                            {
                                resolved_args.push(e);
                            }
                        }

                        // find constructor which matches this
                        if let Ok((constructor_no, args)) =
                            match_constructor_to_args(&base.loc, resolved_args, base_no, ns)
                        {
                            ns.contracts[*contract_no].bases[pos].constructor =
                                Some((constructor_no, args));
                        }
                    }
                }
            }
        }
    }
}

/// Visit base contracts in depth-first post-order
fn visit_bases(contract_no: usize, ns: &ast::Namespace) -> Vec<usize> {
    let mut order = Vec::new();

    fn base<'a>(contract_no: usize, order: &mut Vec<usize>, ns: &'a ast::Namespace) {
        for b in ns.contracts[contract_no].bases.iter().rev() {
            base(b.contract_no, order, ns);
        }

        if !order.contains(&contract_no) {
            order.push(contract_no);
        }
    }

    base(contract_no, &mut order, ns);

    order
}

// Is a contract a base of another contract
pub fn is_base(base: usize, parent: usize, ns: &ast::Namespace) -> bool {
    let bases = &ns.contracts[parent].bases;

    if base == parent || bases.iter().any(|e| e.contract_no == base) {
        return true;
    }

    bases
        .iter()
        .any(|parent| is_base(base, parent.contract_no, ns))
}

/// Layout the contract. We determine the layout of variables
fn layout_contract(contract_no: usize, ns: &mut ast::Namespace) {
    let mut syms: HashMap<String, ast::Symbol> = HashMap::new();
    let mut override_needed: HashMap<String, Vec<(usize, usize)>> = HashMap::new();

    let mut slot = BigInt::zero();

    for base_contract_no in visit_bases(contract_no, ns) {
        // find all syms for this contract
        for ((_, iter_contract_no, name), sym) in &ns.symbols {
            if *iter_contract_no != Some(base_contract_no) {
                continue;
            }

            let mut done = false;

            if let Some(ast::Symbol::Function(ref mut list)) = syms.get_mut(name) {
                if let ast::Symbol::Function(funcs) = sym {
                    list.extend(funcs.to_owned());
                    done = true;
                }
            }

            if !done {
                if let Some(prev) = syms.get(name) {
                    ns.diagnostics.push(ast::Diagnostic::error_with_note(
                        *sym.loc(),
                        format!("already defined ‘{}’", name),
                        *prev.loc(),
                        format!("previous definition of ‘{}’", name),
                    ));
                }
            }

            if !sym.is_private_variable(ns) {
                syms.insert(name.to_owned(), sym.clone());
            }
        }

        for var_no in 0..ns.contracts[base_contract_no].variables.len() {
            if ns.contracts[base_contract_no].variables[var_no].is_storage() {
                ns.contracts[contract_no].layout.push(ast::Layout {
                    slot: slot.clone(),
                    contract_no: base_contract_no,
                    var_no,
                });

                slot += ns.contracts[base_contract_no].variables[var_no]
                    .ty
                    .storage_slots(ns);
            }
        }

        // add functions to our function_table
        for function_no in 0..ns.contracts[base_contract_no].functions.len() {
            let vsignature = ns.contracts[base_contract_no].functions[function_no]
                .vsignature
                .to_owned();

            let cur = &ns.contracts[base_contract_no].functions[function_no];

            if let Some(entry) = override_needed.get(&vsignature) {
                let non_virtual = entry
                    .iter()
                    .filter_map(|(contract_no, function_no)| {
                        let func = &ns.contracts[*contract_no].functions[*function_no];

                        if func.is_virtual {
                            None
                        } else {
                            Some(ast::Note {
                                pos: func.loc,
                                message: format!(
                                    "function ‘{}’ is not specified ‘virtual’",
                                    func.name
                                ),
                            })
                        }
                    })
                    .collect::<Vec<ast::Note>>();

                if !non_virtual.is_empty() {
                    ns.diagnostics.push(ast::Diagnostic::error_with_notes(
                        cur.loc,
                        format!(
                            "function ‘{}’ overrides functions which are not ‘virtual’",
                            cur.name
                        ),
                        non_virtual,
                    ));
                }

                let source_override = entry
                    .iter()
                    .map(|(contract_no, _)| -> &str { &ns.contracts[*contract_no].name })
                    .collect::<Vec<&str>>()
                    .join(",");

                if let Some((loc, override_specified)) = &cur.is_override {
                    if override_specified.is_empty() {
                        ns.diagnostics.push(ast::Diagnostic::error(
                            *loc,
                            format!(
                                "function ‘{}’ should specify override list ‘override({})’",
                                cur.name, source_override
                            ),
                        ));
                    } else {
                        let override_specified: HashSet<usize> =
                            HashSet::from_iter(override_specified.iter().cloned());
                        let override_needed: HashSet<usize> =
                            HashSet::from_iter(entry.iter().map(|(contract_no, _)| *contract_no));

                        // List of contract which should have been specified
                        let missing: Vec<String> = override_needed
                            .difference(&override_specified)
                            .map(|contract_no| ns.contracts[*contract_no].name.to_owned())
                            .collect();

                        if !missing.is_empty() {
                            ns.diagnostics.push(ast::Diagnostic::error(
                                *loc,
                                format!(
                                    "function ‘{}’ missing overrides ‘{}’, specify ‘override({})’",
                                    cur.name,
                                    missing.join(","),
                                    source_override
                                ),
                            ));
                        }

                        // List of contract which should not have been specified
                        let extra: Vec<String> = override_specified
                            .difference(&override_needed)
                            .map(|contract_no| ns.contracts[*contract_no].name.to_owned())
                            .collect();

                        if !extra.is_empty() {
                            ns.diagnostics.push(ast::Diagnostic::error(
                            *loc,
                            format!(
                                "function ‘{}’ includes extraneous overrides ‘{}’, specify ‘override({})’",
                                cur.name,
                                extra.join(","),
                                source_override
                            ),
                        ));
                        }
                    }

                    override_needed.remove(&vsignature);
                } else {
                    ns.diagnostics.push(ast::Diagnostic::error(
                        cur.loc,
                        format!(
                            "function ‘{}’ should specify override list ‘override({})’",
                            cur.name, source_override
                        ),
                    ));
                }
            } else if let Some(prev) = ns.contracts[contract_no].function_table.get(&vsignature) {
                let func_prev = &ns.contracts[prev.0].functions[prev.1];

                if base_contract_no == prev.0 {
                    ns.diagnostics.push(ast::Diagnostic::error_with_note(
                        cur.loc,
                        format!(
                            "function ‘{}’ overrides function in same contract",
                            cur.name
                        ),
                        func_prev.loc,
                        format!("previous definition of ‘{}’", func_prev.name),
                    ));

                    continue;
                }

                if let Some((loc, override_list)) = &cur.is_override {
                    if !func_prev.is_virtual {
                        ns.diagnostics.push(ast::Diagnostic::error_with_note(
                            cur.loc,
                            format!(
                                "function ‘{}’ overrides function which is not virtual",
                                cur.name
                            ),
                            func_prev.loc,
                            format!("previous definition of function ‘{}’", func_prev.name),
                        ));

                        continue;
                    }

                    if !override_list.is_empty() && !override_list.contains(&base_contract_no) {
                        ns.diagnostics.push(ast::Diagnostic::error_with_note(
                            *loc,
                            format!(
                                "function ‘{}’ override list does not contain ‘{}’",
                                cur.name, ns.contracts[prev.0].name
                            ),
                            func_prev.loc,
                            format!("previous definition of function ‘{}’", func_prev.name),
                        ));
                        continue;
                    }
                } else {
                    if let Some(entry) = override_needed.get_mut(&vsignature) {
                        entry.push((base_contract_no, function_no));
                    } else {
                        override_needed.insert(
                            vsignature,
                            vec![(prev.0, prev.1), (base_contract_no, function_no)],
                        );
                    }

                    continue;
                }
            } else if cur.is_override.is_some() {
                ns.diagnostics.push(ast::Diagnostic::error(
                    cur.loc,
                    format!("function ‘{}’ does not override anything", cur.name),
                ));

                continue;
            }

            ns.contracts[contract_no]
                .function_table
                .insert(vsignature, (base_contract_no, function_no, None));
        }
    }

    for list in override_needed.values() {
        let func = &ns.contracts[list[0].0].functions[list[0].1];

        let notes = list
            .iter()
            .skip(1)
            .map(|(contract_no, function_no)| {
                let func = &ns.contracts[*contract_no].functions[*function_no];

                ast::Note {
                    pos: func.loc,
                    message: format!("previous definition of function ‘{}’", func.name),
                }
            })
            .collect();

        ns.diagnostics.push(ast::Diagnostic::error_with_notes(
            func.loc,
            format!(
                "function ‘{}’ with this signature already defined",
                func.name
            ),
            notes,
        ));
    }
}

/// Resolve functions declarations, constructor declarations, and contract variables
/// This returns a list of function bodies to resolve
fn resolve_declarations<'a>(
    def: &'a pt::ContractDefinition,
    file_no: usize,
    contract_no: usize,
    ns: &mut ast::Namespace,
) -> Vec<(usize, usize, &'a pt::FunctionDefinition)> {
    ns.diagnostics.push(ast::Diagnostic::debug(
        def.loc,
        format!("found {} ‘{}’", def.ty, def.name.name),
    ));

    let mut function_no_bodies = Vec::new();
    let mut resolve_bodies = Vec::new();

    // resolve function signatures
    for parts in &def.parts {
        if let pt::ContractPart::FunctionDefinition(ref f) = parts {
            if let Some(function_no) = functions::function_decl(f, file_no, contract_no, ns) {
                if !f.body.is_empty() {
                    resolve_bodies.push((contract_no, function_no, f.as_ref()));
                } else {
                    function_no_bodies.push(function_no);
                }
            }
        }
    }

    match &def.ty {
        pt::ContractTy::Contract(loc) => {
            if !function_no_bodies.is_empty() {
                let notes = function_no_bodies
                    .into_iter()
                    .map(|function_no| ast::Note {
                        pos: ns.contracts[contract_no].functions[function_no].loc,
                        message: format!(
                            "location of function ‘{}’ with no body",
                            ns.contracts[contract_no].functions[function_no].name
                        ),
                    })
                    .collect::<Vec<ast::Note>>();

                ns.diagnostics.push(ast::Diagnostic::error_with_notes(
                    *loc,
                    format!(
                        "contract should be marked ‘abstract contract’ since it has {} functions with no body",
                        notes.len()
                    ),
                    notes,
                ));
            }
        }
        pt::ContractTy::Interface(_) => {
            // no constructor allowed, every function should be declared external and no bodies allowed
            for func in &ns.contracts[contract_no].functions {
                if func.is_constructor() {
                    ns.diagnostics.push(ast::Diagnostic::error(
                        func.loc,
                        "constructor not allowed in an interface".to_string(),
                    ));
                    continue;
                }

                if !func.is_virtual {
                    ns.diagnostics.push(ast::Diagnostic::error(
                        func.loc,
                        "functions can not have bodies in an interface".to_string(),
                    ));
                    continue;
                }

                if !func.is_public() {
                    ns.diagnostics.push(ast::Diagnostic::error(
                        func.loc,
                        "functions must be declared ‘external’ in an interface".to_string(),
                    ));
                    continue;
                }
            }
        }
        _ => (),
    }

    // resolve state variables
    variables::contract_variables(&def, file_no, contract_no, ns);

    resolve_bodies
}

/// Resolve contract functions bodies
fn resolve_bodies(
    bodies: Vec<(usize, usize, &pt::FunctionDefinition)>,
    file_no: usize,
    ns: &mut ast::Namespace,
) -> bool {
    let mut broken = false;

    for (contract_no, function_no, def) in bodies {
        if statements::resolve_function_body(def, file_no, contract_no, function_no, ns).is_err() {
            broken = true;
        }
    }

    broken
}

/// Check if we have arguments for all the base contracts
fn check_base_args(contract_no: usize, ns: &mut ast::Namespace) {
    let contract = &ns.contracts[contract_no];

    if !contract.is_concrete() || contract.have_constructor() {
        // nothing to do, already checked in constructor or has no constructor
        return;
    }

    for base in &contract.bases {
        // do we have constructor arguments
        if base.constructor.is_some() {
            continue;
        }

        // does the contract require arguments
        if ns.contracts[base.contract_no].constructor_needs_arguments() {
            ns.diagnostics.push(ast::Diagnostic::error(
                contract.loc,
                format!(
                    "missing arguments to base contract ‘{}’ constructor",
                    ns.contracts[base.contract_no].name
                ),
            ));
        }
    }

    // Substrate requires one constructor. Ideally we do not create implict things
    // in the ast, but this is required for abi generation which is done of the ast
    if ns.target == Target::Substrate {
        let mut fdecl = ast::Function::new(
            pt::Loc(0, 0, 0),
            contract_no,
            "".to_owned(),
            vec![],
            pt::FunctionTy::Constructor,
            None,
            pt::Visibility::Public(pt::Loc(0, 0, 0)),
            Vec::new(),
            Vec::new(),
            ns,
        );

        fdecl.body = vec![ast::Statement::Return(pt::Loc(0, 0, 0), Vec::new())];

        ns.contracts[contract_no].functions.push(fdecl);
    }
}
