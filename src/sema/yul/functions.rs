// SPDX-License-Identifier: Apache-2.0

use crate::sema::ast::{Namespace, Parameter, Type};
use crate::sema::expression::ExprContext;
use crate::sema::symtable::{LoopScopes, Symtable, VariableInitializer, VariableUsage};
use crate::sema::yul::ast::YulFunction;
use crate::sema::yul::block::resolve_yul_block;
use crate::sema::yul::builtin::{parse_builtin_keyword, yul_unsupported_builtin};
use crate::sema::yul::types::get_type_from_string;
use indexmap::IndexMap;
use solang_parser::diagnostics::{ErrorType, Level, Note};
use solang_parser::pt::YulFunctionDefinition;
use solang_parser::{diagnostics::Diagnostic, pt};
use std::collections::HashMap;
use std::sync::Arc;

/// Saves resolved function headers, so that we can account for function calls, before
/// resolving the function's body
pub struct FunctionHeader {
    pub id: pt::Identifier,
    pub params: Arc<Vec<Parameter<Type>>>,
    pub returns: Arc<Vec<Parameter<Type>>>,
    pub function_no: usize,
    called: bool,
}

/// Keeps track of declared functions and their scope
pub struct FunctionsTable {
    scopes: Vec<HashMap<String, usize>>,
    lookup: IndexMap<String, FunctionHeader>,
    counter: usize,
    offset: usize,
    pub resolved_functions: Vec<YulFunction>,
}

impl FunctionsTable {
    pub fn new(offset: usize) -> FunctionsTable {
        FunctionsTable {
            scopes: vec![],
            lookup: IndexMap::new(),
            offset,
            counter: 0,
            resolved_functions: vec![],
        }
    }

    pub fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn leave_scope(&mut self, ns: &mut Namespace) {
        let scope = self.scopes.pop().unwrap();
        for function_no in scope.values() {
            let header = &self.lookup[*function_no - self.offset];
            if header.called {
                self.resolved_functions[*function_no - self.offset].called = true;
            } else {
                ns.diagnostics.push(Diagnostic::warning(
                    header.id.loc,
                    "yul function has never been used".to_string(),
                ));
            }
        }
    }

    pub fn find(&self, name: &str) -> Option<&FunctionHeader> {
        for scope in self.scopes.iter().rev() {
            if let Some(func_idx) = scope.get(name) {
                return Some(self.lookup.get_index(*func_idx - self.offset).unwrap().1);
            }
        }
        None
    }

    pub fn get(&self, index: usize) -> Option<&FunctionHeader> {
        if let Some(func_data) = self.lookup.get_index(index - self.offset) {
            Some(func_data.1)
        } else {
            None
        }
    }

    pub fn add_function_header(
        &mut self,
        id: &pt::Identifier,
        params: Vec<Parameter<Type>>,
        returns: Vec<Parameter<Type>>,
    ) -> Option<Diagnostic> {
        if let Some(func) = self.find(&id.name) {
            return Some(Diagnostic {
                level: Level::Error,
                ty: ErrorType::DeclarationError,
                loc: id.loc,
                message: format!("function name '{}' is already taken", id.name),
                notes: vec![Note {
                    loc: func.id.loc,
                    message: "previous declaration found here".to_string(),
                }],
            });
        }

        self.scopes
            .last_mut()
            .unwrap()
            .insert(id.name.clone(), self.counter + self.offset);

        self.lookup.insert(
            id.name.clone(),
            FunctionHeader {
                id: id.clone(),
                params: Arc::new(params),
                returns: Arc::new(returns),
                function_no: self.counter + self.offset,
                called: false,
            },
        );

        // Create the space for the function in the vector, so we can assign later.
        self.resolved_functions.push(YulFunction::default());

        self.counter += 1;

        None
    }

    pub fn function_called(&mut self, func_no: usize) {
        self.lookup
            .get_index_mut(func_no - self.offset)
            .unwrap()
            .1
            .called = true;
    }

    /// This function returns a yul function's index in the resolved_functions vector
    pub fn function_index(&self, name: &String) -> Option<usize> {
        self.lookup
            .get(name)
            .map(|header| header.function_no - self.offset)
    }
}

/// Resolve the parameters of a function declaration
fn process_parameters(
    parameters: &[pt::YulTypedIdentifier],
    ns: &mut Namespace,
) -> Vec<Parameter<Type>> {
    let mut params: Vec<Parameter<Type>> = Vec::with_capacity(parameters.len());
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

        params.push(Parameter {
            loc: item.loc,
            ty,
            ty_loc: item.ty.as_ref().map(|ty_id| ty_id.loc),
            indexed: false,
            id: Some(item.id.clone()),
            readonly: false,
            infinite_size: false,
            recursive: false,
            annotation: None,
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
            loc: func_def.id.loc,
            message: format!("function '{}' is already defined", func_def.id.name),
            notes: vec![Note {
                loc: defined_func.id.loc,
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
    context: &mut ExprContext,
    ns: &mut Namespace,
) -> Result<YulFunction, ()> {
    let mut symtable = Symtable::default();
    context.enter_scope();

    let prev_yul_function = context.yul_function;
    context.yul_function = true;

    let mut context = scopeguard::guard(context, |context| {
        context.yul_function = prev_yul_function;
    });

    let function_header = functions_table.find(&func_def.id.name).unwrap();
    let params = function_header.params.clone();
    let returns = function_header.returns.clone();
    let func_no = function_header.function_no;

    for item in &*params {
        let pos = symtable.exclusive_add(
            item.id.as_ref().unwrap(),
            item.ty.clone(),
            ns,
            VariableInitializer::Yul(true),
            VariableUsage::YulLocalVariable,
            None,
            &mut context,
        );
        symtable.arguments.push(pos);
    }

    for item in &*returns {
        if let Some(pos) = symtable.exclusive_add(
            item.id.as_ref().unwrap(),
            item.ty.clone(),
            ns,
            VariableInitializer::Yul(false),
            VariableUsage::YulLocalVariable,
            None,
            &mut context,
        ) {
            // If exclusive add returns None, the return variable's name cannot be used.
            symtable.returns.push(pos);
        }
    }

    let mut loop_scope = LoopScopes::new();

    let (body_block, _) = resolve_yul_block(
        &func_def.body.loc,
        &func_def.body.statements,
        &mut context,
        true,
        &mut loop_scope,
        functions_table,
        &mut symtable,
        ns,
    );

    context.leave_scope(&mut symtable, func_def.loc);

    Ok(YulFunction {
        loc: func_def.loc,
        name: func_def.id.name.clone(),
        params,
        returns,
        body: body_block,
        symtable,
        func_no,
        parent_sol_func: context.function_no,
        called: false,
        cfg_no: 0,
    })
}
