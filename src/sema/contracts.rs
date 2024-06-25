// SPDX-License-Identifier: Apache-2.0

use super::{
    annotions_not_allowed,
    ast::{self},
    diagnostics::Diagnostics,
    expression::{compatible_mutability, ExprContext},
    functions, statements,
    symtable::Symtable,
    using, variables, ContractDefinition,
};
use crate::sema::ast::SolanaAccount;
use crate::sema::expression::constructor::match_constructor_to_args;
use crate::{sema::ast::Namespace, sema::unused_variable::emit_warning_local_variable};
use indexmap::{IndexMap, IndexSet};
use num_bigint::BigInt;
use num_traits::Zero;
use once_cell::unsync::OnceCell;
use solang_parser::diagnostics::{Diagnostic, Note};
use solang_parser::pt::FunctionTy;
use solang_parser::pt::{self, CodeLocation};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::convert::TryInto;
use tiny_keccak::{Hasher, Keccak};

impl ast::Contract {
    /// Create a new contract, abstract contract, interface or library
    pub fn new(
        name: &pt::Identifier,
        ty: pt::ContractTy,
        tags: Vec<ast::Tag>,
        loc: pt::Loc,
    ) -> Self {
        let instantiable = matches!(ty, pt::ContractTy::Contract(_));

        ast::Contract {
            id: name.clone(),
            loc,
            ty,
            bases: Vec::new(),
            using: Vec::new(),
            layout: Vec::new(),
            fixed_layout_size: BigInt::zero(),
            tags,
            functions: Vec::new(),
            all_functions: BTreeMap::new(),
            yul_functions: Vec::new(),
            virtual_functions: HashMap::new(),
            variables: Vec::new(),
            creates: Vec::new(),
            emits_events: Vec::new(),
            initializer: None,
            default_constructor: None,
            cfg: Vec::new(),
            code: OnceCell::new(),
            instantiable,
            program_id: None,
        }
    }

    /// Selector for this contract. This is used by Solana contract bundle
    pub fn selector(&self) -> u32 {
        let mut hasher = Keccak::v256();
        let mut hash = [0u8; 32];
        hasher.update(self.id.name.as_bytes());
        hasher.finalize(&mut hash);

        u32::from_le_bytes(hash[0..4].try_into().unwrap())
    }
}

/// Resolve the following contract
pub fn resolve(contracts: &[ContractDefinition], file_no: usize, ns: &mut ast::Namespace) {
    // we need to resolve declarations first, so we call functions/constructors of
    // contracts before they are declared
    let mut delayed: ResolveLater = Default::default();

    for def in contracts {
        resolve_declarations(def, file_no, ns, &mut delayed);
    }

    // using may use functions declared in contracts
    resolve_using(contracts, file_no, ns);

    // Resolve base contract constructor arguments on contract definition (not constructor definitions)
    resolve_base_args(contracts, file_no, ns);

    // Now we have all the declarations, we can handle base contracts
    for contract in contracts {
        let contract_no = contract.contract_no;

        check_inheritance(contract_no, ns);
        mangle_function_names(contract_no, ns);
        verify_unique_selector(contract_no, ns);
        polkadot_requires_public_functions(contract_no, ns);
        unique_constructor_names(contract_no, ns);
        check_mangled_function_names(contract_no, ns);
    }

    // Now we can resolve the initializers
    variables::resolve_initializers(&delayed.initializers, file_no, ns);

    // Now we can resolve the bodies
    if !resolve_bodies(delayed.function_bodies, file_no, ns) {
        // only if we could resolve all the bodies
        for contract in contracts {
            check_base_args(contract.contract_no, ns);
        }
    }
}

/// Resolve the base contracts list and check for cycles. Returns true if no
/// issues where found.
pub fn resolve_base_contracts(
    contracts: &[ContractDefinition],
    file_no: usize,
    ns: &mut ast::Namespace,
) {
    let mut diagnostics = Diagnostics::default();

    for contract in contracts {
        for base in &contract.base {
            if ns.contracts[contract.contract_no].is_library() {
                ns.diagnostics.push(ast::Diagnostic::error(
                    base.loc,
                    format!(
                        "library '{}' cannot have a base contract",
                        ns.contracts[contract.contract_no].id
                    ),
                ));
                continue;
            }
            let name = &base.name;
            if let Ok(no) = ns.resolve_contract_with_namespace(file_no, name, &mut diagnostics) {
                if no == contract.contract_no {
                    ns.diagnostics.push(ast::Diagnostic::error(
                        name.loc,
                        format!("contract '{name}' cannot have itself as a base contract"),
                    ));
                } else if ns.contracts[contract.contract_no]
                    .bases
                    .iter()
                    .any(|e| e.contract_no == no)
                {
                    ns.diagnostics.push(ast::Diagnostic::error(
                        name.loc,
                        format!(
                            "contract '{}' duplicate base '{}'",
                            ns.contracts[contract.contract_no].id, name
                        ),
                    ));
                } else if is_base(contract.contract_no, no, ns) {
                    ns.diagnostics.push(ast::Diagnostic::error(
                        name.loc,
                        format!(
                            "base '{}' from contract '{}' is cyclic",
                            name, ns.contracts[contract.contract_no].id
                        ),
                    ));
                } else if ns.contracts[contract.contract_no].is_interface()
                    && !ns.contracts[no].is_interface()
                {
                    ns.diagnostics.push(ast::Diagnostic::error(
                        name.loc,
                        format!(
                            "interface '{}' cannot have {} '{}' as a base",
                            ns.contracts[contract.contract_no].id, ns.contracts[no].ty, name
                        ),
                    ));
                } else if ns.contracts[no].is_library() {
                    let contract = &ns.contracts[contract.contract_no];

                    ns.diagnostics.push(ast::Diagnostic::error(
                        name.loc,
                        format!(
                            "library '{}' cannot be used as base contract for {} '{}'",
                            name, contract.ty, contract.id,
                        ),
                    ));
                } else {
                    // We do not resolve the constructor arguments here, since we have not
                    // resolved any variables. This means no constants can be used on base
                    // constructor args, so we delay this until resolve_base_args()
                    ns.contracts[contract.contract_no].bases.push(ast::Base {
                        loc: base.loc,
                        contract_no: no,
                        constructor: None,
                    });
                }
            }
        }
    }

    ns.diagnostics.extend(diagnostics);
}

/// Resolve the base contracts list and check for cycles. Returns true if no
/// issues where found.
fn resolve_base_args(contracts: &[ContractDefinition], file_no: usize, ns: &mut ast::Namespace) {
    let mut diagnostics = Diagnostics::default();

    // for every contract, if we have a base which resolved successfully, resolve any constructor args
    for contract in contracts {
        let mut context = ExprContext {
            file_no,
            contract_no: Some(contract.contract_no),
            ..Default::default()
        };
        context.enter_scope();

        for base in &contract.base {
            let name = &base.name;
            if let Ok(base_no) = ns.resolve_contract_with_namespace(file_no, name, &mut diagnostics)
            {
                if let Some(pos) = ns.contracts[contract.contract_no]
                    .bases
                    .iter()
                    .position(|e| e.contract_no == base_no)
                {
                    if let Some(args) = &base.args {
                        let mut symtable = Symtable::default();

                        // find constructor which matches this
                        if let Ok((Some(constructor_no), args)) = match_constructor_to_args(
                            &base.loc,
                            args,
                            base_no,
                            &mut context,
                            ns,
                            &mut symtable,
                            &mut diagnostics,
                        ) {
                            ns.contracts[contract.contract_no].bases[pos].constructor =
                                Some((constructor_no, args));
                        }
                    }
                }
            }
        }
    }

    ns.diagnostics.extend(diagnostics);
}

impl ast::Namespace {
    /// base contracts in depth-first post-order
    pub fn contract_bases(&self, contract_no: usize) -> Vec<usize> {
        let mut order = Vec::new();

        fn base(contract_no: usize, order: &mut Vec<usize>, ns: &ast::Namespace) {
            for b in ns.contracts[contract_no].bases.iter().rev() {
                base(b.contract_no, order, ns);
            }

            if !order.contains(&contract_no) {
                order.push(contract_no);
            }
        }

        base(contract_no, &mut order, self);

        order
    }
}

// Is a contract a base of another contract
pub fn is_base(base: usize, derived: usize, ns: &ast::Namespace) -> bool {
    let bases = &ns.contracts[derived].bases;

    if base == derived || bases.iter().any(|e| e.contract_no == base) {
        return true;
    }

    bases
        .iter()
        .any(|parent| is_base(base, parent.contract_no, ns))
}

/// Check the inheritance of all functions and other symbols
fn check_inheritance(contract_no: usize, ns: &mut ast::Namespace) {
    let mut function_syms: HashMap<String, ast::Symbol> = HashMap::new();
    let mut variable_syms: HashMap<String, ast::Symbol> = HashMap::new();
    let mut override_needed: BTreeMap<String, Vec<(usize, usize)>> = BTreeMap::new();
    let mut diagnostics = Diagnostics::default();

    for base_contract_no in ns.contract_bases(contract_no) {
        // find file number where contract is defined
        let contract_file_no = ns.contracts[base_contract_no].loc.file_no();

        // find all syms for this contract
        for ((file_no, iter_contract_no, name), sym) in
            ns.variable_symbols.iter().chain(ns.function_symbols.iter())
        {
            if *iter_contract_no != Some(base_contract_no) || *file_no != contract_file_no {
                continue;
            }

            let mut done = false;

            if let Some(ast::Symbol::Function(ref mut list)) = function_syms.get_mut(name) {
                if let ast::Symbol::Function(funcs) = sym {
                    list.extend(funcs.to_owned());
                    done = true;
                }
            }

            if !done {
                if let Some(prev) = variable_syms.get(name).or_else(|| function_syms.get(name)) {
                    // events can be redefined, so allow duplicate event symbols
                    // if a variable has an accessor function (i.e. public) then allow the variable sym,
                    // check for duplicates will be on accessor function
                    if !(prev.has_accessor(ns)
                        || sym.has_accessor(ns)
                        || prev.is_event() && sym.is_event())
                    {
                        diagnostics.push(ast::Diagnostic::error_with_note(
                            sym.loc(),
                            format!("already defined '{name}'"),
                            prev.loc(),
                            format!("previous definition of '{name}'"),
                        ));
                    }
                }
            }

            if !sym.is_private_variable(ns) {
                if let ast::Symbol::Function(_) = sym {
                    function_syms.insert(name.to_owned(), sym.clone());
                } else {
                    variable_syms.insert(name.to_owned(), sym.clone());
                }
            }
        }

        // add functions to our function_table
        for function_no in ns.contracts[base_contract_no].functions.clone() {
            let cur = &ns.functions[function_no];

            let signature = cur.signature.to_owned();

            if let Some(entry) = override_needed.get(&signature) {
                let non_virtual = entry
                    .iter()
                    .filter_map(|(_, function_no)| {
                        let func = &ns.functions[*function_no];

                        if func.is_virtual {
                            None
                        } else {
                            Some(ast::Note {
                                loc: func.loc_prototype,
                                message: format!(
                                    "function '{}' is not specified 'virtual'",
                                    func.id
                                ),
                            })
                        }
                    })
                    .collect::<Vec<ast::Note>>();

                if !non_virtual.is_empty() {
                    diagnostics.push(ast::Diagnostic::error_with_notes(
                        cur.loc_prototype,
                        format!(
                            "function '{}' overrides functions which are not 'virtual'",
                            cur.id
                        ),
                        non_virtual,
                    ));
                }

                let source_override = entry
                    .iter()
                    .map(|(contract_no, _)| -> &str { &ns.contracts[*contract_no].id.name })
                    .collect::<Vec<&str>>()
                    .join(",");

                if let Some((loc, override_specified)) = &cur.is_override {
                    if override_specified.is_empty() && entry.len() > 1 {
                        diagnostics.push(ast::Diagnostic::error(
                            *loc,
                            format!(
                                "function '{}' should specify override list 'override({})'",
                                cur.id, source_override
                            ),
                        ));
                    } else {
                        let override_specified: HashSet<usize> =
                            override_specified.iter().copied().collect();
                        let override_needed: HashSet<usize> =
                            entry.iter().map(|(contract_no, _)| *contract_no).collect();

                        // List of contract which should have been specified
                        let missing: Vec<String> = override_needed
                            .difference(&override_specified)
                            .map(|contract_no| ns.contracts[*contract_no].id.name.to_owned())
                            .collect();

                        if !missing.is_empty() && override_needed.len() >= 2 {
                            diagnostics.push(ast::Diagnostic::error(
                                *loc,
                                format!(
                                    "function '{}' missing overrides '{}', specify 'override({})'",
                                    cur.id,
                                    missing.join(","),
                                    source_override
                                ),
                            ));
                        }

                        // List of contract which should not have been specified
                        let extra: Vec<String> = override_specified
                            .difference(&override_needed)
                            .map(|contract_no| ns.contracts[*contract_no].id.name.to_owned())
                            .collect();

                        if !extra.is_empty() {
                            diagnostics.push(ast::Diagnostic::error(
                                *loc,
                                format!(
                                    "function '{}' includes extraneous overrides '{}', specify 'override({})'",
                                    cur.id,
                                    extra.join(","),
                                    source_override
                                ),
                            ));
                        }
                    }

                    for (_, function_no) in entry {
                        let func = &ns.functions[*function_no];

                        base_function_compatible(func, cur, &mut diagnostics);
                    }

                    override_needed.remove(&signature);
                } else if entry.len() == 1 {
                    let (base_contract_no, function_no) = entry[0];

                    // Solidity 0.5 does not require the override keyword at all, later versions so. Uniswap v2 does
                    // not specify override for implementing interfaces. As a compromise, only require override when
                    // not implementing an interface
                    if !ns.contracts[base_contract_no].is_interface() {
                        diagnostics.push(ast::Diagnostic::error(
                            cur.loc_prototype,
                            format!("function '{}' should specify 'override'", cur.id),
                        ));
                    }

                    let func = &ns.functions[function_no];

                    base_function_compatible(func, cur, &mut diagnostics);

                    override_needed.remove(&signature);
                } else {
                    diagnostics.push(ast::Diagnostic::error(
                        cur.loc_prototype,
                        format!(
                            "function '{}' should specify override list 'override({})'",
                            cur.id, source_override
                        ),
                    ));
                }
            } else {
                let previous_defs = ns.contracts[contract_no]
                    .all_functions
                    .keys()
                    .filter(|function_no| {
                        let func = &ns.functions[**function_no];

                        func.ty != pt::FunctionTy::Constructor && func.signature == signature
                    })
                    .cloned()
                    .collect::<Vec<usize>>();

                if previous_defs.is_empty() && cur.is_override.is_some() {
                    diagnostics.push(ast::Diagnostic::error(
                        cur.loc_prototype,
                        format!("'{}' does not override anything", cur.id),
                    ));
                    continue;
                }

                // a function without body needs an override, if the contract is concrete
                if previous_defs.is_empty()
                    && !cur.has_body
                    && ns.contracts[contract_no].is_concrete()
                {
                    override_needed
                        .insert(signature.clone(), vec![(base_contract_no, function_no)]);
                    continue;
                }

                for prev in previous_defs {
                    let func_prev = &ns.functions[prev];

                    if Some(base_contract_no) == func_prev.contract_no {
                        diagnostics.push(ast::Diagnostic::error_with_note(
                            cur.loc_prototype,
                            format!("function '{}' overrides function in same contract", cur.id),
                            func_prev.loc_prototype,
                            format!("previous definition of '{}'", func_prev.id),
                        ));

                        continue;
                    }

                    if func_prev.ty != cur.ty {
                        diagnostics.push(ast::Diagnostic::error_with_note(
                            cur.loc_prototype,
                            format!("{} '{}' overrides {}", cur.ty, cur.id, func_prev.ty,),
                            func_prev.loc_prototype,
                            format!("previous definition of '{}'", func_prev.id),
                        ));

                        continue;
                    }

                    if func_prev
                        .params
                        .iter()
                        .zip(cur.params.iter())
                        .any(|(a, b)| a.ty != b.ty)
                    {
                        diagnostics.push(ast::Diagnostic::error_with_note(
                            cur.loc_prototype,
                            format!(
                                "{} '{}' overrides {} with different argument types",
                                cur.ty, cur.id, func_prev.ty,
                            ),
                            func_prev.loc_prototype,
                            format!("previous definition of '{}'", func_prev.id),
                        ));

                        continue;
                    }

                    if func_prev
                        .returns
                        .iter()
                        .zip(cur.returns.iter())
                        .any(|(a, b)| a.ty != b.ty)
                    {
                        diagnostics.push(ast::Diagnostic::error_with_note(
                            cur.loc_prototype,
                            format!(
                                "{} '{}' overrides {} with different return types",
                                cur.ty, cur.id, func_prev.ty,
                            ),
                            func_prev.loc_prototype,
                            format!("previous definition of '{}'", func_prev.id),
                        ));

                        continue;
                    }

                    base_function_compatible(func_prev, cur, &mut diagnostics);

                    // if a function needs an override, it was defined in a contract, not outside
                    let prev_contract_no = func_prev.contract_no.unwrap();

                    if let Some((loc, override_list)) = &cur.is_override {
                        if !func_prev.is_virtual {
                            diagnostics.push(ast::Diagnostic::error_with_note(
                                cur.loc_prototype,
                                format!(
                                    "function '{}' overrides function which is not virtual",
                                    cur.id
                                ),
                                func_prev.loc_prototype,
                                format!("previous definition of function '{}'", func_prev.id),
                            ));

                            continue;
                        }

                        if !override_list.is_empty() && !override_list.contains(&prev_contract_no) {
                            diagnostics.push(ast::Diagnostic::error_with_note(
                                *loc,
                                format!(
                                    "function '{}' override list does not contain '{}'",
                                    cur.id, ns.contracts[prev_contract_no].id
                                ),
                                func_prev.loc_prototype,
                                format!("previous definition of function '{}'", func_prev.id),
                            ));
                            continue;
                        }
                    } else if cur.has_body {
                        if let Some(entry) = override_needed.get_mut(&signature) {
                            entry.push((base_contract_no, function_no));
                        } else {
                            override_needed.insert(
                                signature.clone(),
                                vec![(prev_contract_no, prev), (base_contract_no, function_no)],
                            );
                        }
                        continue;
                    }
                }
            }

            if cur.is_override.is_some() || cur.is_virtual {
                ns.contracts[contract_no]
                    .virtual_functions
                    .entry(signature)
                    .or_default()
                    .push(function_no); // there is always at least 1 element in the vector
            }

            ns.contracts[contract_no]
                .all_functions
                .insert(function_no, usize::MAX);
        }
    }

    for list in override_needed.values() {
        let func = &ns.functions[list[0].1];

        // interface or abstract contracts are allowed to have virtual function which are not overriden
        if func.is_virtual && !ns.contracts[contract_no].is_concrete() {
            continue;
        }

        // virtual functions without a body
        if list.len() == 1 {
            let loc = ns.contracts[contract_no].loc;
            match func.ty {
                pt::FunctionTy::Fallback | pt::FunctionTy::Receive => {
                    diagnostics.push(ast::Diagnostic::error_with_note(
                        loc,
                        format!(
                            "contract '{}' missing override for {} function",
                            ns.contracts[contract_no].id, func.ty
                        ),
                        func.loc_prototype,
                        format!("declaration of {} function", func.ty),
                    ));
                }
                _ => diagnostics.push(ast::Diagnostic::error_with_note(
                    loc,
                    format!(
                        "contract '{}' missing override for function '{}'",
                        ns.contracts[contract_no].id, func.id
                    ),
                    func.loc_prototype,
                    format!("declaration of function '{}'", func.id),
                )),
            }

            continue;
        }

        let notes = list
            .iter()
            .skip(1)
            .map(|(_, function_no)| {
                let func = &ns.functions[*function_no];
                ast::Note {
                    loc: func.loc_prototype,
                    message: format!("previous definition of function '{}'", func.id),
                }
            })
            .collect();

        diagnostics.push(ast::Diagnostic::error_with_notes(
            func.loc_prototype,
            format!("function '{}' with this signature already defined", func.id),
            notes,
        ));
    }

    ns.diagnostics.extend(diagnostics);
}

/// Given a contract number, check for function names conflicting with any mangled name.
/// Only applies to public functions.
///
/// Note: In sema we do not care about the function name too much.
/// The mangled name is consumed later by the ABI generation.
fn check_mangled_function_names(contract_no: usize, ns: &mut ast::Namespace) {
    let public_functions: Vec<usize> = ns.contracts[contract_no]
        .all_functions
        .keys()
        .copied()
        .filter(|f| ns.functions[*f].is_public())
        .filter(|f| {
            matches!(
                ns.functions[*f].ty,
                pt::FunctionTy::Function | pt::FunctionTy::Constructor
            )
        })
        .collect();

    for f in &public_functions {
        if let Some(offender) = public_functions
            .iter()
            .find(|other| ns.functions[*f].mangled_name == ns.functions[**other].id.name)
        {
            let f = &ns.functions[*f];
            let message = format!(
                "mangling the symbol of overloaded function '{}' with signature '{}' results in a new symbol '{}' but this symbol already exists",
                &f.id, &f.signature, f.mangled_name
            );
            ns.diagnostics.push(ast::Diagnostic::error_with_note(
                f.loc_prototype,
                message,
                ns.functions[*offender].loc_prototype,
                "this function declaration conflicts with mangled name".into(),
            ))
        }
    }
}

/// A contract on the contracts pallet requires at least one public message
fn polkadot_requires_public_functions(contract_no: usize, ns: &mut ast::Namespace) {
    let contract = &mut ns.contracts[contract_no];

    if ns.target.is_polkadot()
        && !ns.diagnostics.any_errors()
        && contract.is_concrete()
        && !contract.all_functions.keys().any(|func_no| {
            let func = &ns.functions[*func_no];

            match func.ty {
                pt::FunctionTy::Function => func.is_public(),
                pt::FunctionTy::Fallback | pt::FunctionTy::Receive => true,
                _ => false,
            }
        })
    {
        let message = format!("contracts without public storage or functions are not allowed on Polkadot. Consider declaring this contract abstract: 'abstract contract {}'", contract.id);
        contract.instantiable = false;

        ns.diagnostics
            .push(ast::Diagnostic::error(contract.loc, message));
    }
}

/// Constructors and functions are no different pallet contracts.
/// This function checks that all constructors and function names are unique.
/// Overloading (mangled function or constructor names) is taken into account.
fn unique_constructor_names(contract_no: usize, ns: &mut ast::Namespace) {
    if ns.diagnostics.any_errors() {
        return;
    }

    let mut functions = HashMap::new();
    for f in &ns.contracts[contract_no].functions {
        let func = &ns.functions[*f];
        if !func.is_public() {
            continue;
        }

        if let Some(offender) = functions.insert(&func.mangled_name, *f) {
            ns.diagnostics.push(ast::Diagnostic::error_with_note(
                func.loc_prototype,
                format!("Non unique function or constructor name '{}'", &func.id),
                ns.functions[offender].loc_prototype,
                format!("previous declaration of '{}'", &ns.functions[offender].id),
            ))
        }
    }
}

/// Generate diagnostics if function attributes are not compatible with base function
fn base_function_compatible(
    base: &ast::Function,
    func: &ast::Function,
    diagnostics: &mut Diagnostics,
) {
    if !base.is_accessor
        && !func.is_accessor
        && !compatible_mutability(&func.mutability, &base.mutability)
    {
        diagnostics.push(ast::Diagnostic::error_with_note(
            func.loc_prototype,
            format!(
                "mutability '{}' of function '{}' is not compatible with mutability '{}'",
                func.mutability, func.id, base.mutability
            ),
            base.loc_prototype,
            String::from("location of base function"),
        ));
    }

    if !compatible_visibility(&func.visibility, &base.visibility) {
        diagnostics.push(ast::Diagnostic::error_with_note(
            func.loc_prototype,
            format!(
                "visibility '{}' of function '{}' is not compatible with visibility '{}'",
                func.visibility, func.id, base.visibility
            ),
            base.loc_prototype,
            String::from("location of base function"),
        ));
    }

    match (&func.selector, &base.selector) {
        (cur, func) if cur == func => (),
        (Some(cur_selector), Some(func_selector)) => {
            diagnostics.push(ast::Diagnostic::error_with_note(
                cur_selector.0,
                format!(
                    "selector of function '{}' different from base selector",
                    func.id,
                ),
                func_selector.0,
                String::from("location of base function"),
            ));
        }
        (None, Some(func_selector)) => {
            diagnostics.push(ast::Diagnostic::error_with_note(
                func.loc_prototype,
                format!(
                    "selector of function '{}' must match base selector",
                    func.id,
                ),
                func_selector.0,
                String::from("location of base function"),
            ));
        }
        (Some(cur_selector), None) => {
            diagnostics.push(ast::Diagnostic::error_with_note(
                cur_selector.0,
                format!(
                    "base function needs same selector as selector of function '{}'",
                    func.id,
                ),
                base.loc_prototype,
                String::from("location of base function"),
            ));
        }
        // rust compile wants this, already handled in first arm
        (None, None) => (),
    }

    let mut no_correspondence: Vec<(pt::Loc, &String)> = Vec::new();
    let mut incorrect_flag: IndexSet<(pt::Loc, pt::Loc, &String)> = IndexSet::new();
    let func_accounts = &*func.solana_accounts.borrow();
    let base_accounts = &*base.solana_accounts.borrow();
    let mut correct_ordering = true;

    let (correct, func_acc_locations) = check_override_accounts_compatible(
        base_accounts,
        func_accounts,
        &mut no_correspondence,
        &mut incorrect_flag,
        false,
    );
    correct_ordering &= correct;

    let (correct, base_acc_locations) = check_override_accounts_compatible(
        func_accounts,
        base_accounts,
        &mut no_correspondence,
        &mut incorrect_flag,
        true,
    );
    correct_ordering &= correct;

    if !no_correspondence.is_empty() {
        let notes = no_correspondence
            .iter()
            .map(|(loc, account_name)| Note {
                loc: *loc,
                message: format!("corresponding account '{}' is missing", account_name),
            })
            .collect::<Vec<Note>>();

        diagnostics.push(Diagnostic::error_with_notes(
            func.loc_prototype,
            "functions must have the same declared accounts for correct overriding".to_string(),
            notes,
        ));
    }

    if !incorrect_flag.is_empty() {
        for (loc_1, loc_2, account_name) in &incorrect_flag {
            diagnostics.push(Diagnostic::error_with_note(
                *loc_1,
                format!(
                    "account '{}' must be declared with the same annotation for overriding",
                    account_name
                ),
                *loc_2,
                "location of other declaration".to_string(),
            ));
        }
    }

    if !correct_ordering {
        diagnostics.push(Diagnostic::error_with_note(
            func_acc_locations.unwrap(),
            "accounts must be declared in the same order for overriding".to_string(),
            base_acc_locations.unwrap(),
            "location of base function accounts".to_string(),
        ));
    }
}

/// Checks if the accounts from the virtual function and the overriding one are compatible.
/// Returns true if the accounts have been declared in the same order in both functions and
/// the location of all the account declarations.
fn check_override_accounts_compatible<'a>(
    func_accounts: &'a IndexMap<String, SolanaAccount>,
    other_accounts: &'a IndexMap<String, SolanaAccount>,
    no_correspondence: &mut Vec<(pt::Loc, &'a String)>,
    incorrect_flag: &mut IndexSet<(pt::Loc, pt::Loc, &'a String)>,
    reverse: bool,
) -> (bool, Option<pt::Loc>) {
    let mut correct_order = true;
    let mut locations = if let Some((_, acc)) = other_accounts.get_index(0) {
        Some(acc.loc)
    } else {
        None
    };

    for (account_no, (account_name, account_flags)) in other_accounts.iter().enumerate() {
        locations.as_mut().unwrap().union(&account_flags.loc);
        if let Some((other_no, _, other_account)) = func_accounts.get_full(account_name) {
            if other_account.is_signer != account_flags.is_signer
                || other_account.is_writer != account_flags.is_writer
            {
                if reverse {
                    incorrect_flag.insert((other_account.loc, account_flags.loc, account_name));
                } else {
                    incorrect_flag.insert((account_flags.loc, other_account.loc, account_name));
                }
            } else if account_no != other_no {
                correct_order = false;
            }
        } else {
            no_correspondence.push((account_flags.loc, account_name));
        }
    }

    (correct_order, locations)
}

/// Function body which should be resolved.
/// List of function_no, contract_no, and function parse tree
struct DelayedResolveFunction<'a> {
    function_no: usize,
    contract_no: usize,
    function: &'a pt::FunctionDefinition,
    annotations: Vec<&'a pt::Annotation>,
}

#[derive(Default)]

/// Function bodies and state variable initializers can only be resolved once
/// all function prototypes, bases contracts and state variables are resolved.
struct ResolveLater<'a> {
    function_bodies: Vec<DelayedResolveFunction<'a>>,
    initializers: Vec<variables::DelayedResolveInitializer<'a>>,
}

/// Resolve functions declarations, constructor declarations, and contract variables
/// This returns a list of function bodies to resolve
fn resolve_declarations<'a>(
    def: &'a ContractDefinition,
    file_no: usize,
    ns: &mut ast::Namespace,
    delayed: &mut ResolveLater<'a>,
) {
    ns.diagnostics.push(ast::Diagnostic::debug(
        def.loc,
        format!("found {} '{}'", def.ty, def.name.unwrap().name),
    ));

    let mut function_no_bodies = Vec::new();

    // resolve state variables. We may need a constant to resolve the array
    // dimension of a function argument.
    delayed
        .initializers
        .extend(variables::contract_variables(def, file_no, ns));

    // resolve function signatures
    for part in &def.parts {
        if let pt::ContractPart::FunctionDefinition(ref f) = &part.part {
            if let Some(function_no) = functions::contract_function(
                def,
                f,
                &part.doccomments,
                &part.annotations,
                file_no,
                ns,
            ) {
                if f.body.is_some() {
                    delayed.function_bodies.push(DelayedResolveFunction {
                        contract_no: def.contract_no,
                        function_no,
                        function: f.as_ref(),
                        annotations: part.annotations.clone(),
                    });
                } else {
                    function_no_bodies.push(function_no);
                }
            }
        }
    }

    if let pt::ContractTy::Contract(loc) = &def.ty {
        if !function_no_bodies.is_empty() {
            let notes = function_no_bodies
                .into_iter()
                .map(|function_no| ast::Note {
                    loc: ns.functions[function_no].loc_prototype,
                    message: format!(
                        "location of function '{}' with no body",
                        ns.functions[function_no].id
                    ),
                })
                .collect::<Vec<ast::Note>>();

            ns.diagnostics.push(ast::Diagnostic::error_with_notes(
                    *loc,
                    format!(
                        "contract should be marked 'abstract contract' since it has {} functions with no body",
                        notes.len()
                    ),
                    notes,
                ));
        }
    }
}

/// Resolve the using declarations in a contract
fn resolve_using(contracts: &[ContractDefinition], file_no: usize, ns: &mut ast::Namespace) {
    for contract in contracts {
        for part in &contract.parts {
            if let pt::ContractPart::Using(using) = &part.part {
                annotions_not_allowed(&part.annotations, "using", ns);

                if let Ok(using) = using::using_decl(using, file_no, Some(contract.contract_no), ns)
                {
                    ns.contracts[contract.contract_no].using.push(using);
                }
            }
        }
    }
}

/// Resolve contract functions bodies
fn resolve_bodies(
    bodies: Vec<DelayedResolveFunction>,
    file_no: usize,
    ns: &mut ast::Namespace,
) -> bool {
    let mut broken = false;

    for DelayedResolveFunction {
        contract_no,
        function_no,
        function,
        annotations,
    } in bodies
    {
        if statements::resolve_function_body(
            function,
            &annotations,
            file_no,
            Some(contract_no),
            function_no,
            ns,
        )
        .is_err()
        {
            broken = true;
        } else if !ns.diagnostics.any_errors() {
            for variable in ns.functions[function_no].symtable.vars.values() {
                if let Some(warning) = emit_warning_local_variable(variable, ns) {
                    ns.diagnostics.push(warning);
                }
            }
        }
    }

    broken
}

#[derive(Debug)]
pub struct BaseOrModifier<'a> {
    pub loc: &'a pt::Loc,
    pub defined_constructor_no: Option<usize>,
    pub calling_constructor_no: usize,
    pub args: &'a Vec<ast::Expression>,
}

// walk the list of base contracts and collect all the base constructor arguments
pub fn collect_base_args<'a>(
    contract_no: usize,
    constructor_no: Option<usize>,
    base_args: &mut BTreeMap<usize, BaseOrModifier<'a>>,
    diagnostics: &mut Diagnostics,
    ns: &'a ast::Namespace,
) {
    let contract = &ns.contracts[contract_no];

    if let Some(defined_constructor_no) = constructor_no {
        let constructor = &ns.functions[defined_constructor_no];

        for (base_no, (loc, constructor_no, args)) in &constructor.bases {
            if let Some(prev_args) = base_args.get(base_no) {
                diagnostics.push(ast::Diagnostic::error_with_note(
                    *loc,
                    format!(
                        "duplicate argument for base contract '{}'",
                        ns.contracts[*base_no].id
                    ),
                    *prev_args.loc,
                    format!(
                        "previous argument for base contract '{}'",
                        ns.contracts[*base_no].id
                    ),
                ));
            } else {
                base_args.insert(
                    *base_no,
                    BaseOrModifier {
                        loc,
                        defined_constructor_no: Some(defined_constructor_no),
                        calling_constructor_no: *constructor_no,
                        args,
                    },
                );

                collect_base_args(*base_no, Some(*constructor_no), base_args, diagnostics, ns);
            }
        }
    }

    for base in &contract.bases {
        if let Some((constructor_no, args)) = &base.constructor {
            if let Some(prev_args) = base_args.get(&base.contract_no) {
                diagnostics.push(ast::Diagnostic::error_with_note(
                    base.loc,
                    format!(
                        "duplicate argument for base contract '{}'",
                        ns.contracts[base.contract_no].id
                    ),
                    *prev_args.loc,
                    format!(
                        "previous argument for base contract '{}'",
                        ns.contracts[base.contract_no].id
                    ),
                ));
            } else {
                base_args.insert(
                    base.contract_no,
                    BaseOrModifier {
                        loc: &base.loc,
                        defined_constructor_no: None,
                        calling_constructor_no: *constructor_no,
                        args,
                    },
                );

                collect_base_args(
                    base.contract_no,
                    Some(*constructor_no),
                    base_args,
                    diagnostics,
                    ns,
                );
            }
        } else {
            collect_base_args(
                base.contract_no,
                ns.contracts[base.contract_no].no_args_constructor(ns),
                base_args,
                diagnostics,
                ns,
            );
        }
    }
}

/// Check if we have arguments for all the base contracts
fn check_base_args(contract_no: usize, ns: &mut ast::Namespace) {
    let contract = &ns.contracts[contract_no];

    if !contract.is_concrete() {
        return;
    }

    let mut diagnostics = Diagnostics::default();

    let base_args_needed = ns
        .contract_bases(contract_no)
        .into_iter()
        .filter(|base_no| {
            *base_no != contract_no && ns.contracts[*base_no].constructor_needs_arguments(ns)
        })
        .collect::<Vec<usize>>();

    if !contract.constructors(ns).is_empty() {
        for constructor_no in contract
            .functions
            .iter()
            .filter(|function_no| ns.functions[**function_no].is_constructor())
        {
            let mut base_args = BTreeMap::new();

            collect_base_args(
                contract_no,
                Some(*constructor_no),
                &mut base_args,
                &mut diagnostics,
                ns,
            );

            for base_no in &base_args_needed {
                if !base_args.contains_key(base_no) {
                    diagnostics.push(ast::Diagnostic::error(
                        contract.loc,
                        format!(
                            "missing arguments to base contract '{}' constructor",
                            ns.contracts[*base_no].id
                        ),
                    ));
                }
            }
        }
    } else {
        let mut base_args = BTreeMap::new();

        collect_base_args(contract_no, None, &mut base_args, &mut diagnostics, ns);

        for base_no in &base_args_needed {
            if !base_args.contains_key(base_no) {
                diagnostics.push(ast::Diagnostic::error(
                    contract.loc,
                    format!(
                        "missing arguments to base contract '{}' constructor",
                        ns.contracts[*base_no].id
                    ),
                ));
            }
        }
    }

    ns.diagnostics.extend(diagnostics);
}

/// Compare two visibility levels
fn compatible_visibility(left: &pt::Visibility, right: &pt::Visibility) -> bool {
    matches!(
        (left, right),
        // public and external are compatible with each other, otherwise the have to be the same
        (
            pt::Visibility::Public(_) | pt::Visibility::External(_),
            pt::Visibility::Public(_) | pt::Visibility::External(_)
        ) | (pt::Visibility::Internal(_), pt::Visibility::Internal(_))
            | (pt::Visibility::Private(_), pt::Visibility::Private(_))
    )
}

/// This function checks which function names must be mangled given a contract.
/// Mangling happens when there is more than one function with the same name in the given `contract_no`.
fn mangle_function_names(contract_no: usize, ns: &mut Namespace) {
    let mut repeated_names: HashMap<String, usize> = HashMap::new();

    for func_no in ns.contracts[contract_no].all_functions.keys() {
        let function = &ns.functions[*func_no];

        let not_callable = !function.is_public()
            && (function.ty != pt::FunctionTy::Function
                || function.ty != pt::FunctionTy::Constructor);

        if function.is_override.is_some() || not_callable {
            continue;
        }

        if let Some(old_no) = repeated_names.insert(function.id.name.clone(), *func_no) {
            ns.functions[old_no]
                .mangled_name_contracts
                .insert(contract_no);
            ns.functions[*func_no]
                .mangled_name_contracts
                .insert(contract_no);
        }
    }
}

/// This check guarantees that each public Solidity function has a unique selector.
fn verify_unique_selector(contract_no: usize, ns: &mut Namespace) {
    let mut selectors: HashMap<Vec<u8>, usize> = HashMap::new();
    let mut diagnostics: Vec<Diagnostic> = Vec::new();
    for func_no in ns.contracts[contract_no].all_functions.keys() {
        let func = &ns.functions[*func_no];

        let selector = func.selector(ns, &contract_no);
        let selector_len = ns.target.selector_length();

        // On Solana, concrete contracts have selectors of 8 bytes
        if ns.contracts[contract_no].is_concrete() && selector.len() != selector_len as usize {
            let loc = if let Some((loc, _)) = &func.selector {
                loc
            } else {
                &func.loc_prototype
            };

            diagnostics.push(ast::Diagnostic::error(
                *loc,
                format!(
                    "function '{}' selector must be {} bytes rather than {} bytes",
                    func.id,
                    selector_len,
                    selector.len()
                ),
            ));
        }

        if let Some(other_func_no) = selectors.get(&selector) {
            let other = &ns.functions[*other_func_no];

            if other.signature != func.signature
                && func.ty != FunctionTy::Constructor
                && func.is_public()
                && other.is_public()
            {
                diagnostics.push(ast::Diagnostic::error_with_note(
                    func.loc_prototype,
                    format!(
                        "{} '{}' selector is the same as {} '{}'",
                        func.ty, func.id, other.ty, other.id
                    ),
                    other.loc_prototype,
                    format!("definition of {} '{}'", other.ty, other.id),
                ));
            }
        } else {
            selectors.insert(selector, *func_no);
        }
    }

    ns.diagnostics.append(&mut diagnostics);
}
