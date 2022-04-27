use super::ast::{
    Builtin, DestructureField, Diagnostic, Expression, Function, Mutability, Namespace, Statement,
    Type,
};
use crate::parser::pt;
use crate::sema::ast::RetrieveType;
use crate::sema::yul::ast::{YulExpression, YulStatement};
use crate::sema::Recurse;

/// check state mutability
pub fn mutability(file_no: usize, ns: &mut Namespace) {
    if !ns.diagnostics.any_errors() {
        for func in &ns.functions {
            if func.loc.file_no() != file_no {
                continue;
            }

            let diagnostics = check_mutability(func, ns);

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
                    "function declared '{}' but this expression writes to state",
                    self.func.mutability
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
                    "function declared '{}' but this expression reads from state",
                    self.func.mutability
                ),
            ));
        }

        self.does_read_state = true;
    }

    //TODO: This is a temporary solution while inline assembly is not supported in codegen
    fn has_yul(&mut self, loc: &pt::Loc) {
        self.diagnostics.push(Diagnostic::error(
            *loc,
            "inline assembly is not yet supported".to_string(),
        ));
    }
}

fn check_mutability(func: &Function, ns: &Namespace) -> Vec<Diagnostic> {
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
        Mutability::Pure(_) => (),
        Mutability::View(_) => {
            state.can_read_state = true;
        }
        Mutability::Payable(_) | Mutability::Nonpayable(_) => {
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

            let contract_no = func
                .contract_no
                .expect("only functions in contracts have modifiers");

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

                recurse_statements(&func.body, ns, &mut state);
            }
        }
    }

    recurse_statements(&func.body, ns, &mut state);

    if pt::FunctionTy::Function == func.ty && !func.is_accessor {
        if !state.does_write_state && !state.does_read_state {
            match func.mutability {
                Mutability::Payable(_) | Mutability::Pure(_) => (),
                Mutability::Nonpayable(_) => {
                    state.diagnostics.push(Diagnostic::warning(
                        func.loc,
                        "function can be declared 'pure'".to_string(),
                    ));
                }
                _ => {
                    state.diagnostics.push(Diagnostic::warning(
                        func.loc,
                        format!(
                            "function declared '{}' can be declared 'pure'",
                            func.mutability
                        ),
                    ));
                }
            }
        }

        if !state.does_write_state && state.does_read_state && func.mutability.is_default() {
            state.diagnostics.push(Diagnostic::warning(
                func.loc,
                "function can be declared 'view'".to_string(),
            ));
        }
    }

    state.diagnostics
}

fn recurse_statements(stmts: &[Statement], ns: &Namespace, state: &mut StateCheck) {
    for stmt in stmts.iter() {
        match stmt {
            Statement::Block { statements, .. } => {
                recurse_statements(statements, ns, state);
            }
            Statement::VariableDecl(_, _, _, Some(expr)) => {
                expr.recurse(state, read_expression);
            }
            Statement::VariableDecl(_, _, _, None) => (),
            Statement::If(_, _, expr, then_, else_) => {
                expr.recurse(state, read_expression);
                recurse_statements(then_, ns, state);
                recurse_statements(else_, ns, state);
            }
            Statement::DoWhile(_, _, body, expr) | Statement::While(_, _, expr, body) => {
                expr.recurse(state, read_expression);
                recurse_statements(body, ns, state);
            }
            Statement::For {
                init,
                cond,
                next,
                body,
                ..
            } => {
                recurse_statements(init, ns, state);
                if let Some(cond) = cond {
                    cond.recurse(state, read_expression);
                }
                recurse_statements(next, ns, state);
                recurse_statements(body, ns, state);
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
            Statement::Return(_, None) => {}
            Statement::Return(_, Some(expr)) => {
                expr.recurse(state, read_expression);
            }
            Statement::TryCatch(_, _, try_catch) => {
                try_catch.expr.recurse(state, read_expression);
                recurse_statements(&try_catch.ok_stmt, ns, state);
                for (_, _, s) in &try_catch.errors {
                    recurse_statements(s, ns, state);
                }
                recurse_statements(&try_catch.catch_stmt, ns, state);
            }
            Statement::Emit { loc, .. } => state.write(loc),
            Statement::Break(_) | Statement::Continue(_) | Statement::Underscore(_) => (),
            Statement::Assembly(inline_assembly, _) => {
                state.has_yul(&inline_assembly.loc);
                for function_no in inline_assembly.functions.start..inline_assembly.functions.end {
                    recurse_yul_statements(&ns.yul_functions[function_no].body, state);
                }
                recurse_yul_statements(&inline_assembly.body, state);
            }
        }
    }
}

fn read_expression(expr: &Expression, state: &mut StateCheck) -> bool {
    match expr {
        Expression::PreIncrement(_, _, _, expr)
        | Expression::PreDecrement(_, _, _, expr)
        | Expression::PostIncrement(_, _, _, expr)
        | Expression::PostDecrement(_, _, _, expr) => {
            expr.recurse(state, write_expression);
        }
        Expression::Assign(_, _, left, right) => {
            right.recurse(state, read_expression);
            left.recurse(state, write_expression);
        }
        Expression::StorageArrayLength { loc, .. }
        | Expression::StorageVariable(loc, _, _, _)
        | Expression::StorageLoad(loc, _, _) => state.read(loc),
        Expression::Subscript(loc, _, ty, ..) | Expression::Variable(loc, ty, _)
            if ty.is_contract_storage() =>
        {
            state.read(loc)
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
        | Expression::Builtin(loc, _, Builtin::Balance, _)
        | Expression::Builtin(loc, _, Builtin::Random, _) => state.read(loc),
        Expression::Builtin(loc, _, Builtin::PayableSend, _)
        | Expression::Builtin(loc, _, Builtin::PayableTransfer, _)
        | Expression::Builtin(loc, _, Builtin::SelfDestruct, _) => state.write(loc),
        Expression::Builtin(loc, _, Builtin::ArrayPush, args)
        | Expression::Builtin(loc, _, Builtin::ArrayPop, args)
            if args[0].ty().is_contract_storage() =>
        {
            state.write(loc)
        }

        Expression::Constructor { loc, .. } => {
            state.write(loc);
        }
        Expression::ExternalFunctionCall { loc, function, .. }
        | Expression::InternalFunctionCall { loc, function, .. } => match function.ty() {
            Type::ExternalFunction { mutability, .. }
            | Type::InternalFunction { mutability, .. } => {
                match mutability {
                    Mutability::Nonpayable(_) | Mutability::Payable(_) => state.write(loc),
                    Mutability::View(_) => state.read(loc),
                    Mutability::Pure(_) => (),
                };
            }
            _ => unreachable!(),
        },
        Expression::ExternalFunctionCallRaw { loc, .. } => {
            if state.ns.target.is_substrate() {
                state.write(loc)
            } else {
                state.read(loc)
            }
        }
        _ => {
            return true;
        }
    }
    false
}

fn write_expression(expr: &Expression, state: &mut StateCheck) -> bool {
    match expr {
        Expression::StructMember(loc, _, expr, _) | Expression::Subscript(loc, _, _, expr, _) => {
            if expr.ty().is_contract_storage() {
                state.write(loc);
                return false;
            }
        }
        Expression::Variable(loc, ty, _) => {
            if ty.is_contract_storage() && !expr.ty().is_contract_storage() {
                state.write(loc);
                return false;
            }
        }
        Expression::StorageVariable(loc, _, _, _) => {
            state.write(loc);
            return false;
        }
        _ => (),
    }

    true
}

fn recurse_yul_statements(stmts: &[YulStatement], state: &mut StateCheck) {
    for stmt in stmts {
        match stmt {
            YulStatement::FunctionCall(_, _, _, args) => {
                for arg in args {
                    arg.recurse(state, check_expression_mutability_yul);
                }
            }
            YulStatement::BuiltInCall(loc, _, builtin_ty, args) => {
                if builtin_ty.read_state() {
                    state.read(loc);
                } else if builtin_ty.modify_state() {
                    state.write(loc);
                }
                for arg in args {
                    arg.recurse(state, check_expression_mutability_yul);
                }
            }
            YulStatement::Block(block) => {
                recurse_yul_statements(&block.body, state);
            }
            YulStatement::Assignment(_, _, _, value)
            | YulStatement::VariableDeclaration(_, _, _, Some(value)) => {
                value.recurse(state, check_expression_mutability_yul);
            }
            YulStatement::IfBlock(_, _, condition, block) => {
                condition.recurse(state, check_expression_mutability_yul);
                recurse_yul_statements(&block.body, state);
            }
            YulStatement::Switch {
                condition,
                cases,
                default,
                ..
            } => {
                condition.recurse(state, check_expression_mutability_yul);
                for item in cases {
                    item.condition
                        .recurse(state, check_expression_mutability_yul);
                    recurse_yul_statements(&item.block.body, state);
                }

                if let Some(block) = default {
                    recurse_yul_statements(&block.body, state);
                }
            }
            YulStatement::For {
                init_block,
                condition,
                post_block,
                execution_block,
                ..
            } => {
                recurse_yul_statements(&init_block.body, state);
                condition.recurse(state, check_expression_mutability_yul);
                recurse_yul_statements(&post_block.body, state);
                recurse_yul_statements(&execution_block.body, state);
            }

            _ => (),
        }
    }
}

fn check_expression_mutability_yul(expr: &YulExpression, state: &mut StateCheck) -> bool {
    match expr {
        YulExpression::BuiltInCall(loc, builtin_ty, _) => {
            if builtin_ty.read_state() {
                state.read(loc);
            } else if builtin_ty.modify_state() {
                state.write(loc);
            }
            true
        }
        YulExpression::FunctionCall(..) => true,
        _ => false,
    }
}
