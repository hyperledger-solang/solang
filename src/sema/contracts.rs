use inkwell::OptimizationLevel;
use num_bigint::BigInt;
use num_traits::Zero;
use output::{Note, Output};
use parser::pt;
use Target;

use super::ast;
use super::functions;
use super::statements;
use super::variables;
use codegen::cfg::ControlFlowGraph;
use emit;

impl ast::Contract {
    pub fn new(name: &str, ty: pt::ContractTy, loc: pt::Loc) -> Self {
        ast::Contract {
            name: name.to_owned(),
            loc,
            ty,
            inherit: Vec::new(),
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

/// Resolve the inheritance list and check for cycles
pub fn resolve_inheritance(
    contracts: &[(usize, &pt::ContractDefinition)],
    file_no: usize,
    ns: &mut ast::Namespace,
) {
    for (contract_no, def) in contracts {
        for name in &def.inherits {
            match ns.resolve_contract(file_no, name) {
                Some(no) => {
                    if no == *contract_no {
                        ns.diagnostics.push(Output::error(
                            name.loc,
                            format!("contract ‘{}’ cannot inherit itself", name.name),
                        ));
                    } else {
                        ns.contracts[*contract_no].inherit.push(no);
                    }
                }
                None => {
                    ns.diagnostics.push(Output::error(
                        name.loc,
                        format!("contract ‘{}’ not found", name.name),
                    ));
                }
            }
        }
    }

    // Check the inheritance of a contract
    fn cyclic(no: &usize, set: &[usize], ns: &ast::Namespace) -> bool {
        if set.contains(no) {
            return true;
        }

        set.iter()
            .any(|c| cyclic(no, &ns.contracts[*c].inherit, ns))
    }

    for (contract_no, _) in contracts {
        if cyclic(contract_no, &ns.contracts[*contract_no].inherit, ns) {
            let c = &ns.contracts[*contract_no];
            ns.diagnostics.push(Output::error(
                c.loc,
                format!("contract ‘{}’ inheritance is cyclic", c.name),
            ));
        }
    }
}

/// Resolve functions declarations, constructor declarations, and contract variables
pub fn resolve_declarations(
    def: &pt::ContractDefinition,
    file_no: usize,
    contract_no: usize,
    target: Target,
    ns: &mut ast::Namespace,
) -> bool {
    ns.diagnostics.push(Output::info(
        def.loc,
        format!("found {} ‘{}’", def.ty, def.name.name),
    ));

    let mut broken = false;
    let mut virtual_functions = Vec::new();

    // resolve function signatures
    for (i, parts) in def.parts.iter().enumerate() {
        if let pt::ContractPart::FunctionDefinition(ref f) = parts {
            match functions::function_decl(f, i, file_no, contract_no, ns) {
                Some(function_no) => {
                    if ns.contracts[contract_no].functions[function_no].is_virtual {
                        virtual_functions.push(function_no);
                    }
                }
                None => {
                    broken = true;
                }
            }
        }
    }

    match &def.ty {
        pt::ContractTy::Contract(loc) => {
            if !virtual_functions.is_empty() {
                let notes = virtual_functions
                    .into_iter()
                    .map(|function_no| Note {
                        pos: ns.contracts[contract_no].functions[function_no].loc,
                        message: format!(
                            "location of ‘virtual’ function ‘{}’",
                            ns.contracts[contract_no].functions[function_no].name
                        ),
                    })
                    .collect::<Vec<Note>>();

                ns.diagnostics.push(Output::error_with_notes(
                    *loc,
                    format!(
                        "contract should be marked ‘abstract contract’ since it has {} virtual functions",
                        notes.len()
                    ),
                    notes,
                ));

                broken = true;
            }
        }
        pt::ContractTy::Interface(_) => {
            // no constructor allowed, every function should be declared external and no bodies allowed
            for func in &ns.contracts[contract_no].functions {
                if func.is_constructor() {
                    ns.diagnostics.push(Output::error(
                        func.loc,
                        "constructor not allowed in an interface".to_string(),
                    ));
                    broken = true;
                    continue;
                }

                if !func.is_virtual {
                    ns.diagnostics.push(Output::error(
                        func.loc,
                        "functions can not have bodies in an interface".to_string(),
                    ));

                    broken = true;
                    continue;
                }

                if !func.is_public() {
                    ns.diagnostics.push(Output::error(
                        func.loc,
                        "functions must be declared ‘external’ in an interface".to_string(),
                    ));

                    broken = true;
                    continue;
                }
            }
        }
        _ => (),
    }

    // resolve state variables
    if variables::contract_variables(&def, file_no, contract_no, ns) {
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
            pt::Loc(0, 0, 0),
            "".to_owned(),
            vec![],
            pt::FunctionTy::Constructor,
            None,
            None,
            pt::Visibility::Public(pt::Loc(0, 0, 0)),
            Vec::new(),
            Vec::new(),
            ns,
        );

        fdecl.body = vec![ast::Statement::Return(pt::Loc(0, 0, 0), Vec::new())];

        ns.contracts[contract_no].functions.push(fdecl);
    }

    broken
}

/// Resolve contract functions bodies
pub fn resolve_bodies(
    def: &pt::ContractDefinition,
    file_no: usize,
    contract_no: usize,
    ns: &mut ast::Namespace,
) -> bool {
    let mut broken = false;

    // resolve function bodies
    for f in 0..ns.contracts[contract_no].functions.len() {
        if let Some(ast_index) = ns.contracts[contract_no].functions[f].ast_index {
            if let pt::ContractPart::FunctionDefinition(ref ast_f) = def.parts[ast_index] {
                if statements::resolve_function_body(ast_f, file_no, contract_no, f, ns).is_err() {
                    broken = true;
                }
            }
        }
    }

    broken
}
