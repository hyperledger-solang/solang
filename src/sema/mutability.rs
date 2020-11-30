use super::ast::{
    Builtin, DestructureField, Diagnostic, Expression, Function, Namespace, Statement, Type,
};
use crate::parser::pt;

/// check state mutablity
pub fn mutablity(file_no: usize, ns: &mut Namespace) {
    for contract_no in 0..ns.contracts.len() {
        if ns.contracts[contract_no].loc.0 != file_no {
            continue;
        }

        for func_no in ns.contracts[contract_no].functions.iter() {
            let diagnostics = check_mutability(contract_no, *func_no, ns);

            ns.diagnostics.extend(diagnostics);
        }
    }
}

/// While we recurse through the AST, maintain some state
struct StateCheck<'a> {
    diagnostics: Vec<Diagnostic>,
    does_read_state: bool,
    does_write_state: bool,
    can_read_state: bool,
    can_write_state: bool,
    func: &'a Function,
    ns: &'a Namespace,
}

impl<'a> StateCheck<'a> {
    fn write(&mut self, loc: &pt::Loc) {
        if !self.can_write_state {
            self.diagnostics.push(Diagnostic::error(
                *loc,
                format!(
                    "function declared ‘{}’ but this expression writes to state",
                    self.func.print_mutability()
                ),
            ));
        }

        self.does_write_state = true;
    }

    fn read(&mut self, loc: &pt::Loc) {
        if !self.can_read_state {
            self.diagnostics.push(Diagnostic::error(
                *loc,
                format!(
                    "function declared ‘{}’ but this expression reads from state",
                    self.func.print_mutability()
                ),
            ));
        }

        self.does_read_state = true;
    }
}

fn check_mutability(contract_no: usize, function_no: usize, ns: &Namespace) -> Vec<Diagnostic> {
    let func = &ns.functions[function_no];

    if func.is_virtual {
        return Vec::new();
    }

    let mut state = StateCheck {
        diagnostics: Vec::new(),
        does_read_state: false,
        does_write_state: false,
        can_write_state: false,
        can_read_state: false,
        func,
        ns,
    };

    match func.mutability {
        Some(pt::StateMutability::Pure(_)) => (),
        Some(pt::StateMutability::Constant(_)) | Some(pt::StateMutability::View(_)) => {
            state.can_read_state = true;
        }
        Some(pt::StateMutability::Payable(_)) | None => {
            state.can_read_state = true;
            state.can_write_state = true;
        }
    };

    for arg in &func.modifiers {
        if let Expression::InternalFunctionCall { function, args, .. } = &arg {
            // check the arguments to the modifiers
            for arg in args {
                arg.recurse(&mut state, read_expression);
            }

            // check the modifier itself
            if let Expression::InternalFunction {
                function_no,
                signature,
                ..
            } = function.as_ref()
            {
                let function_no = if let Some(signature) = signature {
                    state.ns.contracts[contract_no].virtual_functions[signature]
                } else {
                    *function_no
                };

                // modifiers do not have mutability, bases or modifiers itself
                let func = &ns.functions[function_no];

                recurse_statements(&func.body, &mut state);
            }
        }
    }

    recurse_statements(&func.body, &mut state);

    if pt::FunctionTy::Function == func.ty {
        if !state.does_write_state && !state.does_read_state {
            match func.mutability {
                Some(pt::StateMutability::Payable(_)) | Some(pt::StateMutability::Pure(_)) => (),
                _ => {
                    state.diagnostics.push(Diagnostic::warning(
                        func.loc,
                        format!(
                            "function declared ‘{}’ can be declared ‘pure’",
                            func.print_mutability()
                        ),
                    ));
                }
            }
        }

        if !state.does_write_state && state.does_read_state && func.mutability.is_none() {
            state.diagnostics.push(Diagnostic::warning(
                func.loc,
                "function declared can be declared ‘view’".to_string(),
            ));
        }
    }

    state.diagnostics
}

fn recurse_statements(stmts: &[Statement], state: &mut StateCheck) {
    for stmt in stmts.iter() {
        match stmt {
            Statement::VariableDecl(_, _, _, Some(expr)) => {
                expr.recurse(state, read_expression);
            }
            Statement::VariableDecl(_, _, _, None) => (),
            Statement::If(_, _, expr, then_, else_) => {
                expr.recurse(state, read_expression);
                recurse_statements(then_, state);
                recurse_statements(else_, state);
            }
            Statement::DoWhile(_, _, body, expr) | Statement::While(_, _, expr, body) => {
                expr.recurse(state, read_expression);
                recurse_statements(body, state);
            }
            Statement::For {
                init,
                cond,
                next,
                body,
                ..
            } => {
                recurse_statements(init, state);
                if let Some(cond) = cond {
                    cond.recurse(state, read_expression);
                }
                recurse_statements(next, state);
                recurse_statements(body, state);
            }
            Statement::Expression(_, _, expr) => {
                expr.recurse(state, read_expression);
            }
            Statement::Delete(loc, _, _) => state.write(loc),
            Statement::Destructure(_, fields, expr) => {
                // This is either a list or internal/external function call
                expr.recurse(state, read_expression);

                for field in fields {
                    if let DestructureField::Expression(expr) = field {
                        expr.recurse(state, write_expression);
                    }
                }
            }
            Statement::Return(_, exprs) => {
                for e in exprs {
                    e.recurse(state, read_expression);
                }
            }
            Statement::TryCatch {
                expr,
                ok_stmt,
                error,
                catch_stmt,
                ..
            } => {
                expr.recurse(state, read_expression);
                recurse_statements(ok_stmt, state);
                if let Some((_, _, s)) = error {
                    recurse_statements(s, state);
                }
                recurse_statements(catch_stmt, state);
            }
            Statement::Emit { loc, .. } => state.write(loc),
            Statement::Break(_) | Statement::Continue(_) | Statement::Underscore(_) => (),
        }
    }
}

fn read_expression(expr: &Expression, state: &mut StateCheck) -> bool {
    match expr {
        Expression::PreIncrement(_, _, expr)
        | Expression::PreDecrement(_, _, expr)
        | Expression::PostIncrement(_, _, expr)
        | Expression::PostDecrement(_, _, expr) => {
            expr.recurse(state, write_expression);
        }
        Expression::Assign(_, _, left, right) => {
            right.recurse(state, read_expression);
            left.recurse(state, write_expression);
        }
        Expression::StorageBytesLength(loc, _)
        | Expression::StorageBytesSubscript(loc, _, _)
        | Expression::StorageVariable(loc, _, _, _)
        | Expression::StorageLoad(loc, _, _) => state.read(loc),
        Expression::Variable(loc, ty, _) if ty.is_contract_storage() => state.read(loc),
        Expression::StorageBytesPush(loc, _, _) | Expression::StorageBytesPop(loc, _) => {
            state.write(loc);
        }
        Expression::Builtin(loc, _, Builtin::GetAddress, _)
        | Expression::Builtin(loc, _, Builtin::BlockNumber, _)
        | Expression::Builtin(loc, _, Builtin::Timestamp, _)
        | Expression::Builtin(loc, _, Builtin::BlockCoinbase, _)
        | Expression::Builtin(loc, _, Builtin::BlockDifficulty, _)
        | Expression::Builtin(loc, _, Builtin::BlockHash, _)
        | Expression::Builtin(loc, _, Builtin::Sender, _)
        | Expression::Builtin(loc, _, Builtin::Origin, _)
        | Expression::Builtin(loc, _, Builtin::Gasleft, _)
        | Expression::Builtin(loc, _, Builtin::Gasprice, _)
        | Expression::Builtin(loc, _, Builtin::GasLimit, _)
        | Expression::Builtin(loc, _, Builtin::TombstoneDeposit, _)
        | Expression::Builtin(loc, _, Builtin::MinimumBalance, _)
        | Expression::Builtin(loc, _, Builtin::Random, _) => state.read(loc),
        Expression::Builtin(loc, _, Builtin::PayableSend, _)
        | Expression::Builtin(loc, _, Builtin::PayableTransfer, _)
        | Expression::Builtin(loc, _, Builtin::ArrayPush, _)
        | Expression::Builtin(loc, _, Builtin::ArrayPop, _)
        | Expression::Builtin(loc, _, Builtin::BytesPush, _)
        | Expression::Builtin(loc, _, Builtin::BytesPop, _)
        | Expression::Builtin(loc, _, Builtin::SelfDestruct, _) => state.write(loc),
        Expression::Constructor { loc, .. } => {
            state.write(loc);
        }
        Expression::ExternalFunctionCall { loc, function, .. }
        | Expression::InternalFunctionCall { loc, function, .. } => match function.ty() {
            Type::ExternalFunction { mutability, .. }
            | Type::InternalFunction { mutability, .. } => {
                match mutability {
                    None | Some(pt::StateMutability::Payable(_)) => state.write(loc),
                    Some(pt::StateMutability::View(_)) | Some(pt::StateMutability::Constant(_)) => {
                        state.read(loc)
                    }
                    Some(pt::StateMutability::Pure(_)) => (),
                };
            }
            _ => unreachable!(),
        },
        _ => {
            return true;
        }
    }
    false
}

fn write_expression(expr: &Expression, state: &mut StateCheck) -> bool {
    if let Expression::StorageVariable(loc, _, _, _) = expr {
        state.write(loc);
        false
    } else if let Expression::Variable(loc, ty, _) = expr {
        if ty.is_contract_storage() {
            state.write(loc);
            false
        } else {
            true
        }
    } else {
        true
    }
}
