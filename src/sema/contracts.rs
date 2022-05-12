use crate::parser::pt;
use crate::parser::pt::CodeLocation;
use num_bigint::BigInt;
use num_traits::Zero;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::convert::TryInto;
use tiny_keccak::{Hasher, Keccak};

use super::{
    ast,
    expression::{compatible_mutability, match_constructor_to_args, ExprContext},
    functions, statements,
    symtable::Symtable,
    tags::parse_doccomments,
    using, variables,
};
#[cfg(feature = "llvm")]
use crate::emit;
use crate::sema::unused_variable::emit_warning_local_variable;

impl ast::Contract {
    /// Create a new contract, abstract contract, interface or library
    pub fn new(name: &str, ty: pt::ContractTy, tags: Vec<ast::Tag>, loc: pt::Loc) -> Self {
        ast::Contract {
            name: name.to_owned(),
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
            sends_events: Vec::new(),
            initializer: None,
            default_constructor: None,
            cfg: Vec::new(),
            code: Vec::new(),
        }
    }

    /// Generate contract code for this contract
    #[cfg(feature = "llvm")]
    pub fn emit<'a>(
        &'a self,
        ns: &'a ast::Namespace,
        context: &'a inkwell::context::Context,
        filename: &'a str,
        opt: inkwell::OptimizationLevel,
        math_overflow_check: bool,
    ) -> emit::Binary {
        emit::Binary::build(context, self, ns, filename, opt, math_overflow_check)
    }

    /// Selector for this contract. This is used by Solana contract bundle
    pub fn selector(&self) -> u32 {
        let mut hasher = Keccak::v256();
        let mut hash = [0u8; 32];
        hasher.update(self.name.as_bytes());
        hasher.finalize(&mut hash);

        u32::from_le_bytes(hash[0..4].try_into().unwrap())
    }
}

/// Resolve the following contract
pub fn resolve(
    contracts: &[(usize, &pt::ContractDefinition)],
    file_no: usize,
    ns: &mut ast::Namespace,
) {
    resolve_using(contracts, file_no, ns);

    // we need to resolve declarations first, so we call functions/constructors of
    // contracts before they are declared
    let mut delayed: ResolveLater = Default::default();

    for (contract_no, def) in contracts {
        resolve_declarations(def, file_no, *contract_no, ns, &mut delayed);
    }

    // Resolve base contract constructor arguments on contract definition (not constructor definitions)
    resolve_base_args(contracts, file_no, ns);

    // Now we have all the declarations, we can handle base contracts
    for (contract_no, _) in contracts {
        check_inheritance(*contract_no, ns);
    }

    // Now we can resolve the initializers
    variables::resolve_initializers(&delayed.initializers, file_no, ns);

    // Now we can resolve the bodies
    if !resolve_bodies(delayed.function_bodies, file_no, ns) {
        // only if we could resolve all the bodies
        for (contract_no, _) in contracts {
            check_base_args(*contract_no, ns);
        }
    }
}

/// Resolve the base contracts list and check for cycles. Returns true if no
/// issues where found.
pub fn resolve_base_contracts(
    contracts: &[(usize, &pt::ContractDefinition)],
    file_no: usize,
    ns: &mut ast::Namespace,
) {
    let mut diagnostics = Vec::new();

    for (contract_no, def) in contracts {
        for base in &def.base {
            if ns.contracts[*contract_no].is_library() {
                ns.diagnostics.push(ast::Diagnostic::error(
                    base.loc,
                    format!(
                        "library '{}' cannot have a base contract",
                        ns.contracts[*contract_no].name
                    ),
                ));
                continue;
            }
            let name = &base.name;
            if let Ok(no) = ns.resolve_contract_with_namespace(file_no, name, &mut diagnostics) {
                if no == *contract_no {
                    ns.diagnostics.push(ast::Diagnostic::error(
                        name.loc,
                        format!("contract '{}' cannot have itself as a base contract", name),
                    ));
                } else if ns.contracts[*contract_no]
                    .bases
                    .iter()
                    .any(|e| e.contract_no == no)
                {
                    ns.diagnostics.push(ast::Diagnostic::error(
                        name.loc,
                        format!(
                            "contract '{}' duplicate base '{}'",
                            ns.contracts[*contract_no].name, name
                        ),
                    ));
                } else if is_base(*contract_no, no, ns) {
                    ns.diagnostics.push(ast::Diagnostic::error(
                        name.loc,
                        format!(
                            "base '{}' from contract '{}' is cyclic",
                            name, ns.contracts[*contract_no].name
                        ),
                    ));
                } else if ns.contracts[*contract_no].is_interface()
                    && !ns.contracts[no].is_interface()
                {
                    ns.diagnostics.push(ast::Diagnostic::error(
                        name.loc,
                        format!(
                            "interface '{}' cannot have {} '{}' as a base",
                            ns.contracts[*contract_no].name, ns.contracts[no].ty, name
                        ),
                    ));
                } else if ns.contracts[no].is_library() {
                    let contract = &ns.contracts[*contract_no];

                    ns.diagnostics.push(ast::Diagnostic::error(
                        name.loc,
                        format!(
                            "library '{}' cannot be used as base contract for {} '{}'",
                            name, contract.ty, contract.name,
                        ),
                    ));
                } else {
                    // We do not resolve the constructor arguments here, since we have not
                    // resolved any variables. This means no constants can be used on base
                    // constructor args, so we delay this until resolve_base_args()
                    ns.contracts[*contract_no].bases.push(ast::Base {
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
fn resolve_base_args(
    contracts: &[(usize, &pt::ContractDefinition)],
    file_no: usize,
    ns: &mut ast::Namespace,
) {
    let mut diagnostics = Vec::new();

    // for every contract, if we have a base which resolved successfully, resolve any constructor args
    for (contract_no, def) in contracts {
        let context = ExprContext {
            function_no: None,
            contract_no: Some(*contract_no),
            file_no,
            unchecked: false,
            constant: false,
            lvalue: false,
            yul_function: false,
        };

        for base in &def.base {
            let name = &base.name;
            if let Ok(base_no) = ns.resolve_contract_with_namespace(file_no, name, &mut diagnostics)
            {
                if let Some(pos) = ns.contracts[*contract_no]
                    .bases
                    .iter()
                    .position(|e| e.contract_no == base_no)
                {
                    if let Some(args) = &base.args {
                        let mut symtable = Symtable::new();

                        // find constructor which matches this
                        if let Ok((Some(constructor_no), args)) = match_constructor_to_args(
                            &base.loc,
                            args,
                            base_no,
                            &context,
                            ns,
                            &mut symtable,
                            &mut diagnostics,
                        ) {
                            ns.contracts[*contract_no].bases[pos].constructor =
                                Some((constructor_no, args));
                        }
                    }
                }
            }
        }
    }

    ns.diagnostics.extend(diagnostics);
}

/// Visit base contracts in depth-first post-order
pub fn visit_bases(contract_no: usize, ns: &ast::Namespace) -> Vec<usize> {
    let mut order = Vec::new();

    fn base(contract_no: usize, order: &mut Vec<usize>, ns: &ast::Namespace) {
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

/// Check the inheritance of all functions and other symbols
fn check_inheritance(contract_no: usize, ns: &mut ast::Namespace) {
    let mut function_syms: HashMap<String, ast::Symbol> = HashMap::new();
    let mut variable_syms: HashMap<String, ast::Symbol> = HashMap::new();
    let mut override_needed: BTreeMap<String, Vec<(usize, usize)>> = BTreeMap::new();

    for base_contract_no in visit_bases(contract_no, ns) {
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
                        ns.diagnostics.push(ast::Diagnostic::error_with_note(
                            sym.loc(),
                            format!("already defined '{}'", name),
                            prev.loc(),
                            format!("previous definition of '{}'", name),
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
                                pos: func.loc,
                                message: format!(
                                    "function '{}' is not specified 'virtual'",
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
                            "function '{}' overrides functions which are not 'virtual'",
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
                    if override_specified.is_empty() && entry.len() > 1 {
                        ns.diagnostics.push(ast::Diagnostic::error(
                            *loc,
                            format!(
                                "function '{}' should specify override list 'override({})'",
                                cur.name, source_override
                            ),
                        ));
                    } else {
                        let override_specified: HashSet<usize> =
                            override_specified.iter().cloned().collect();
                        let override_needed: HashSet<usize> =
                            entry.iter().map(|(contract_no, _)| *contract_no).collect();

                        // List of contract which should have been specified
                        let missing: Vec<String> = override_needed
                            .difference(&override_specified)
                            .map(|contract_no| ns.contracts[*contract_no].name.to_owned())
                            .collect();

                        if !missing.is_empty() && override_needed.len() >= 2 {
                            ns.diagnostics.push(ast::Diagnostic::error(
                                *loc,
                                format!(
                                    "function '{}' missing overrides '{}', specify 'override({})'",
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
                                    "function '{}' includes extraneous overrides '{}', specify 'override({})'",
                                    cur.name,
                                    extra.join(","),
                                    source_override
                                ),
                            ));
                        }
                    }

                    for (_, function_no) in entry {
                        let func = &ns.functions[*function_no];

                        if !func.is_accessor
                            && !cur.is_accessor
                            && !compatible_mutability(&cur.mutability, &func.mutability)
                        {
                            ns.diagnostics.push(ast::Diagnostic::error_with_note(
                                cur.loc,
                                format!("mutability '{}' of function '{}' is not compatible with mutability '{}'", cur.mutability, cur.name, func.mutability),
                                func.loc,
                                String::from("location of base function")
                            ));
                        }

                        if !compatible_visibility(&cur.visibility, &func.visibility) {
                            ns.diagnostics.push(ast::Diagnostic::error_with_note(
                                cur.loc,
                                format!("visibility '{}' of function '{}' is not compatible with visibility '{}'", cur.visibility, cur.name, func.visibility),
                                func.loc,
                                String::from("location of base function")
                            ));
                        }
                    }

                    override_needed.remove(&signature);
                } else if entry.len() == 1 {
                    let (base_contract_no, function_no) = entry[0];

                    // Solidity 0.5 does not require the override keyword at all, later versions so. Uniswap v2 does
                    // not specify override for implementing interfaces. As a compromise, only require override when
                    // not implementing an interface
                    if !ns.contracts[base_contract_no].is_interface() {
                        ns.diagnostics.push(ast::Diagnostic::error(
                            cur.loc,
                            format!("function '{}' should specify 'override'", cur.name),
                        ));
                    }

                    let func = &ns.functions[function_no];

                    if !func.is_accessor
                        && !cur.is_accessor
                        && !compatible_mutability(&cur.mutability, &func.mutability)
                    {
                        ns.diagnostics.push(ast::Diagnostic::error_with_note(
                            cur.loc,
                            format!("mutability '{}' of function '{}' is not compatible with mutability '{}'", cur.mutability, cur.name, func.mutability),
                            func.loc,
                            String::from("location of base function")
                        ));
                    }

                    if !compatible_visibility(&cur.visibility, &func.visibility) {
                        ns.diagnostics.push(ast::Diagnostic::error_with_note(
                            cur.loc,
                            format!("visibility '{}' of function '{}' is not compatible with visibility '{}'", cur.visibility, cur.name, func.visibility),
                            func.loc,
                            String::from("location of base function")
                        ));
                    }

                    override_needed.remove(&signature);
                } else {
                    ns.diagnostics.push(ast::Diagnostic::error(
                        cur.loc,
                        format!(
                            "function '{}' should specify override list 'override({})'",
                            cur.name, source_override
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
                    ns.diagnostics.push(ast::Diagnostic::error(
                        cur.loc,
                        format!("function '{}' does not override anything", cur.name),
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

                for prev in previous_defs.into_iter() {
                    let func_prev = &ns.functions[prev];

                    if Some(base_contract_no) == func_prev.contract_no {
                        ns.diagnostics.push(ast::Diagnostic::error_with_note(
                            cur.loc,
                            format!(
                                "function '{}' overrides function in same contract",
                                cur.name
                            ),
                            func_prev.loc,
                            format!("previous definition of '{}'", func_prev.name),
                        ));

                        continue;
                    }

                    if func_prev.ty != cur.ty {
                        ns.diagnostics.push(ast::Diagnostic::error_with_note(
                            cur.loc,
                            format!("{} '{}' overrides {}", cur.ty, cur.name, func_prev.ty,),
                            func_prev.loc,
                            format!("previous definition of '{}'", func_prev.name),
                        ));

                        continue;
                    }

                    if func_prev
                        .params
                        .iter()
                        .zip(cur.params.iter())
                        .any(|(a, b)| a.ty != b.ty)
                    {
                        ns.diagnostics.push(ast::Diagnostic::error_with_note(
                            cur.loc,
                            format!(
                                "{} '{}' overrides {} with different argument types",
                                cur.ty, cur.name, func_prev.ty,
                            ),
                            func_prev.loc,
                            format!("previous definition of '{}'", func_prev.name),
                        ));

                        continue;
                    }

                    if func_prev
                        .returns
                        .iter()
                        .zip(cur.returns.iter())
                        .any(|(a, b)| a.ty != b.ty)
                    {
                        ns.diagnostics.push(ast::Diagnostic::error_with_note(
                            cur.loc,
                            format!(
                                "{} '{}' overrides {} with different return types",
                                cur.ty, cur.name, func_prev.ty,
                            ),
                            func_prev.loc,
                            format!("previous definition of '{}'", func_prev.name),
                        ));

                        continue;
                    }

                    if !func_prev.is_accessor
                        && !cur.is_accessor
                        && !compatible_mutability(&cur.mutability, &func_prev.mutability)
                    {
                        ns.diagnostics.push(ast::Diagnostic::error_with_note(
                            cur.loc,
                            format!("mutability '{}' of function '{}' is not compatible with mutability '{}'", cur.mutability, cur.name, func_prev.mutability),
                            func_prev.loc,
                            String::from("location of base function")
                        ));
                    }

                    if !compatible_visibility(&cur.visibility, &func_prev.visibility) {
                        ns.diagnostics.push(ast::Diagnostic::error_with_note(
                            cur.loc,
                            format!("visibility '{}' of function '{}' is not compatible with visibility '{}'", cur.visibility, cur.name, func_prev.visibility),
                            func_prev.loc,
                            String::from("location of base function")
                        ));
                    }

                    // if a function needs an override, it was defined in a contract, not outside
                    let prev_contract_no = func_prev.contract_no.unwrap();

                    if let Some((loc, override_list)) = &cur.is_override {
                        if !func_prev.is_virtual {
                            ns.diagnostics.push(ast::Diagnostic::error_with_note(
                                cur.loc,
                                format!(
                                    "function '{}' overrides function which is not virtual",
                                    cur.name
                                ),
                                func_prev.loc,
                                format!("previous definition of function '{}'", func_prev.name),
                            ));

                            continue;
                        }

                        if !override_list.is_empty() && !override_list.contains(&prev_contract_no) {
                            ns.diagnostics.push(ast::Diagnostic::error_with_note(
                                *loc,
                                format!(
                                    "function '{}' override list does not contain '{}'",
                                    cur.name, ns.contracts[prev_contract_no].name
                                ),
                                func_prev.loc,
                                format!("previous definition of function '{}'", func_prev.name),
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
                    .insert(signature, function_no);
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
                    ns.diagnostics.push(ast::Diagnostic::error_with_note(
                        loc,
                        format!(
                            "contract '{}' missing override for '{}' function",
                            ns.contracts[contract_no].name, func.ty
                        ),
                        func.loc,
                        format!("declaration of '{}' function", func.ty),
                    ));
                }
                _ => ns.diagnostics.push(ast::Diagnostic::error_with_note(
                    loc,
                    format!(
                        "contract '{}' missing override for function '{}'",
                        ns.contracts[contract_no].name, func.name
                    ),
                    func.loc,
                    format!("declaration of function '{}'", func.name),
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
                    pos: func.loc,
                    message: format!("previous definition of function '{}'", func.name),
                }
            })
            .collect();

        ns.diagnostics.push(ast::Diagnostic::error_with_notes(
            func.loc,
            format!(
                "function '{}' with this signature already defined",
                func.name
            ),
            notes,
        ));
    }
}

/// Function body which should be resolved.
/// List of function_no, contract_no, and function parse tree
struct DelayedResolveFunction<'a> {
    function_no: usize,
    contract_no: usize,
    function: &'a pt::FunctionDefinition,
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
    def: &'a pt::ContractDefinition,
    file_no: usize,
    contract_no: usize,
    ns: &mut ast::Namespace,
    delayed: &mut ResolveLater<'a>,
) {
    ns.diagnostics.push(ast::Diagnostic::debug(
        def.loc,
        format!("found {} '{}'", def.ty, def.name.name),
    ));

    let mut function_no_bodies = Vec::new();
    let mut doccomments = Vec::new();

    // resolve state variables. We may need a constant to resolve the array
    // dimension of a function argument.
    delayed
        .initializers
        .extend(variables::contract_variables(def, file_no, contract_no, ns));

    // resolve function signatures
    for parts in &def.parts {
        match parts {
            pt::ContractPart::FunctionDefinition(ref f) => {
                let tags = parse_doccomments(&doccomments);
                doccomments.clear();

                if let Some(function_no) =
                    functions::contract_function(def, f, &tags, file_no, contract_no, ns)
                {
                    if f.body.is_some() {
                        delayed.function_bodies.push(DelayedResolveFunction {
                            contract_no,
                            function_no,
                            function: f.as_ref(),
                        });
                    } else {
                        function_no_bodies.push(function_no);
                    }
                }
            }
            pt::ContractPart::DocComment(doccomment) => doccomments.push(doccomment),
            _ => doccomments.clear(),
        }
    }

    if let pt::ContractTy::Contract(loc) = &def.ty {
        if !function_no_bodies.is_empty() {
            let notes = function_no_bodies
                .into_iter()
                .map(|function_no| ast::Note {
                    pos: ns.functions[function_no].loc,
                    message: format!(
                        "location of function '{}' with no body",
                        ns.functions[function_no].name
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
}

/// Resolve the using declarations in a contract
fn resolve_using(
    contracts: &[(usize, &pt::ContractDefinition)],
    file_no: usize,
    ns: &mut ast::Namespace,
) {
    for (contract_no, def) in contracts {
        for part in &def.parts {
            if let pt::ContractPart::Using(using) = part {
                if let Ok(using) = using::using_decl(using, file_no, Some(*contract_no), ns) {
                    ns.contracts[*contract_no].using.push(using);
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
    } in bodies
    {
        if statements::resolve_function_body(function, file_no, Some(contract_no), function_no, ns)
            .is_err()
        {
            broken = true;
        } else if !ns.diagnostics.any_errors() {
            for variable in ns.functions[function_no].symtable.vars.values() {
                if let Some(warning) = emit_warning_local_variable(variable) {
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
    diagnostics: &mut BTreeSet<ast::Diagnostic>,
    ns: &'a ast::Namespace,
) {
    let contract = &ns.contracts[contract_no];

    if let Some(defined_constructor_no) = constructor_no {
        let constructor = &ns.functions[defined_constructor_no];

        for (base_no, (loc, constructor_no, args)) in &constructor.bases {
            if let Some(prev_args) = base_args.get(base_no) {
                diagnostics.insert(ast::Diagnostic::error_with_note(
                    *loc,
                    format!(
                        "duplicate argument for base contract '{}'",
                        ns.contracts[*base_no].name
                    ),
                    *prev_args.loc,
                    format!(
                        "previous argument for base contract '{}'",
                        ns.contracts[*base_no].name
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
                diagnostics.insert(ast::Diagnostic::error_with_note(
                    base.loc,
                    format!(
                        "duplicate argument for base contract '{}'",
                        ns.contracts[base.contract_no].name
                    ),
                    *prev_args.loc,
                    format!(
                        "previous argument for base contract '{}'",
                        ns.contracts[base.contract_no].name
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

    let mut diagnostics = BTreeSet::new();
    let base_args_needed = visit_bases(contract_no, ns)
        .into_iter()
        .filter(|base_no| {
            *base_no != contract_no && ns.contracts[*base_no].constructor_needs_arguments(ns)
        })
        .collect::<Vec<usize>>();

    if contract.have_constructor(ns) {
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
                    diagnostics.insert(ast::Diagnostic::error(
                        contract.loc,
                        format!(
                            "missing arguments to base contract '{}' constructor",
                            ns.contracts[*base_no].name
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
                diagnostics.insert(ast::Diagnostic::error(
                    contract.loc,
                    format!(
                        "missing arguments to base contract '{}' constructor",
                        ns.contracts[*base_no].name
                    ),
                ));
            }
        }
    }

    ns.diagnostics.extend(diagnostics.into_iter().collect());
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
