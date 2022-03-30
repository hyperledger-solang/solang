use crate::ast::{Namespace, Type};
use crate::sema::expression::ExprContext;
use crate::sema::symtable::{LoopScopes, Symtable, VariableInitializer, VariableUsage};
use crate::sema::yul::ast::{YulFunction, YulFunctionParameter};
use crate::sema::yul::block::process_statements;
use crate::sema::yul::builtin::{parse_builtin_keyword, yul_unsupported_builtin};
use crate::sema::yul::types::get_type_from_string;
use solang_parser::diagnostics::{ErrorType, Level, Note};
use solang_parser::pt::YulFunctionDefinition;
use solang_parser::{pt, Diagnostic};
use std::collections::{HashMap, LinkedList};
use std::sync::Arc;

/// Saves resolved function headers, so that we can account for function calls, before
/// resolving the function's body
pub struct FunctionHeader {
    pub id: pt::Identifier,
    pub params: Arc<Vec<YulFunctionParameter>>,
    pub returns: Arc<Vec<YulFunctionParameter>>,
    pub function_no: usize,
    called: bool,
}

/// Keeps track of declared functions and their scope
pub struct FunctionsTable {
    scopes: LinkedList<HashMap<String, usize>>,
    lookup: Vec<FunctionHeader>,
    counter: usize,
    pub resolved_functions: Vec<YulFunction>,
}

impl FunctionsTable {
    pub fn new() -> FunctionsTable {
        FunctionsTable {
            scopes: LinkedList::new(),
            lookup: vec![],
            counter: 0,
            resolved_functions: vec![],
        }
    }

    pub fn new_scope(&mut self) {
        self.scopes.push_back(HashMap::new());
    }

    pub fn leave_scope(&mut self, ns: &mut Namespace) {
        let scope = self.scopes.pop_back().unwrap();
        for function_no in scope.values() {
            let header = &self.lookup[*function_no];
            if header.called {
                self.resolved_functions[*function_no].called = true;
            } else {
                ns.diagnostics.push(Diagnostic::warning(
                    header.id.loc,
                    "yul function has never been used".to_string(),
                ));
            }
        }
    }

    pub fn find(&self, name: &str) -> Option<&FunctionHeader> {
        for scope in &self.scopes {
            if let Some(func_idx) = scope.get(name) {
                return Some(self.lookup.get(*func_idx).unwrap());
            }
        }
        None
    }

    pub fn get_params_and_returns(
        &self,
        name: &str,
    ) -> (
        Arc<Vec<YulFunctionParameter>>,
        Arc<Vec<YulFunctionParameter>>,
    ) {
        let header = self.find(name).unwrap();
        (header.params.clone(), header.returns.clone())
    }

    pub fn get(&self, index: usize) -> Option<&FunctionHeader> {
        self.lookup.get(index)
    }

    pub fn add_function_header(
        &mut self,
        id: &pt::Identifier,
        params: Vec<YulFunctionParameter>,
        returns: Vec<YulFunctionParameter>,
    ) -> Option<Diagnostic> {
        if let Some(func) = self.find(&id.name) {
            return Some(Diagnostic {
                level: Level::Error,
                ty: ErrorType::DeclarationError,
                pos: id.loc,
                message: format!("function name '{}' is already taken", id.name),
                notes: vec![Note {
                    pos: func.id.loc,
                    message: "previous declaration found here".to_string(),
                }],
            });
        }

        self.scopes
            .back_mut()
            .unwrap()
            .insert(id.name.clone(), self.counter);

        self.lookup.push(FunctionHeader {
            id: id.clone(),
            params: Arc::new(params),
            returns: Arc::new(returns),
            function_no: self.counter,
            called: false,
        });
        self.counter += 1;

        None
    }

    pub fn function_called(&mut self, func_no: usize) {
        self.lookup.get_mut(func_no).unwrap().called = true;
    }
}

/// Resolve the parameters of a function declaration
fn process_parameters(
    parameters: &[pt::YulTypedIdentifier],
    ns: &mut Namespace,
) -> Vec<YulFunctionParameter> {
    let mut params: Vec<YulFunctionParameter> = Vec::with_capacity(parameters.len());
    for item in parameters {
        let ty = match &item.ty {
            Some(identifier) => {
                if let Some(solang_type) = get_type_from_string(&identifier.name) {
                    solang_type
                } else {
                    ns.diagnostics.push(Diagnostic::error(
                        identifier.loc,
                        format!("unrecognized yul type: {}", identifier.name),
                    ));

                    Type::Uint(256)
                }
            }
            None => Type::Uint(256),
        };

        params.push(YulFunctionParameter {
            loc: item.loc,
            id: item.id.clone(),
            ty,
        });
    }

    params
}

/// Resolve the function header of a declaration and add it to the functions table
pub(crate) fn process_function_header(
    func_def: &YulFunctionDefinition,
    functions_table: &mut FunctionsTable,
    ns: &mut Namespace,
) {
    if let Some(defined_func) = functions_table.find(&func_def.id.name) {
        ns.diagnostics.push(Diagnostic {
            level: Level::Error,
            ty: ErrorType::DeclarationError,
            pos: func_def.id.loc,
            message: format!("function '{}' is already defined", func_def.id.name),
            notes: vec![Note {
                pos: defined_func.id.loc,
                message: "found definition here".to_string(),
            }],
        });
        return;
    } else if parse_builtin_keyword(&func_def.id.name).is_some()
        || yul_unsupported_builtin(&func_def.id.name)
    {
        ns.diagnostics.push(Diagnostic::error(
            func_def.loc,
            format!(
                "function '{}' is a built-in function and cannot be redefined",
                func_def.id.name
            ),
        ));
        return;
    } else if func_def.id.name.starts_with("verbatim") {
        ns.diagnostics.push(Diagnostic::error(
            func_def.id.loc,
            "the prefix 'verbatim' is reserved for verbatim functions".to_string(),
        ));
        return;
    }

    let params = process_parameters(&func_def.params, ns);
    let returns = process_parameters(&func_def.returns, ns);

    if let Some(diagnostic) = functions_table.add_function_header(&func_def.id, params, returns) {
        ns.diagnostics.push(diagnostic);
    }
}

/// Semantic analysis of function definitions
pub(crate) fn resolve_function_definition(
    func_def: &pt::YulFunctionDefinition,
    functions_table: &mut FunctionsTable,
    context: &ExprContext,
    ns: &mut Namespace,
) -> Result<YulFunction, ()> {
    let mut symtable = Symtable::new();
    let mut local_ctx = context.clone();
    local_ctx.yul_function = true;
    functions_table.new_scope();

    let (params, returns) = functions_table.get_params_and_returns(&func_def.id.name);

    for item in &*params {
        let _ = symtable.exclusive_add(
            &item.id,
            item.ty.clone(),
            ns,
            VariableInitializer::Yul(true),
            VariableUsage::YulLocalVariable,
            None,
        );
    }

    for item in &*returns {
        let _ = symtable.exclusive_add(
            &item.id,
            item.ty.clone(),
            ns,
            VariableInitializer::Yul(false),
            VariableUsage::YulLocalVariable,
            None,
        );
    }

    let mut loop_scope = LoopScopes::new();

    let (body, _) = process_statements(
        &func_def.body.statements,
        &local_ctx,
        true,
        &mut symtable,
        &mut loop_scope,
        functions_table,
        ns,
    );

    functions_table.leave_scope(ns);
    Ok(YulFunction {
        loc: func_def.loc,
        name: func_def.id.name.clone(),
        params,
        returns,
        body,
        symtable,
        called: false,
    })
}
