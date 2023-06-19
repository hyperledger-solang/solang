// SPDX-License-Identifier: Apache-2.0

use super::{
    ast::{
        Builtin, CallTy, DestructureField, Diagnostic, Expression, Function, Mutability, Namespace,
        RetrieveType, Statement, Type,
    },
    yul::ast::{YulExpression, YulStatement},
    Recurse,
};
use solang_parser::{helpers::CodeLocation, pt};

#[derive(PartialEq, PartialOrd)]
enum Access {
    None,
    Read,
    Write,
    Value,
}

impl Access {
    fn increase_to(&mut self, other: Access) {
        if *self < other {
            *self = other;
        }
    }
}

/// check state mutability
pub fn mutability(file_no: usize, ns: &mut Namespace) {
    if !ns.diagnostics.any_errors() {
        for func in &ns.functions {
            if func.loc.try_file_no() != Some(file_no) || func.ty == pt::FunctionTy::Modifier {
                continue;
            }

            let mut diagnostics = check_mutability(func, ns);

            ns.diagnostics.append(&mut diagnostics);
        }
    }
}

/// While we recurse through the AST, maintain some state
struct StateCheck<'a> {
    diagnostics: Vec<Diagnostic>,
    declared_access: Access,
    required_access: Access,
    func: &'a Function,
    modifier: Option<pt::Loc>,
    ns: &'a Namespace,
}

impl<'a> StateCheck<'a> {
    fn value(&mut self, loc: &pt::Loc) {
        if self.declared_access != Access::Value {
            if let Some(modifier_loc) = &self.modifier {
                self.diagnostics.push(Diagnostic::error_with_note(
                    *modifier_loc,
                    format!(
                        "function declared '{}' but modifier accesses value sent, which is only allowed for payable functions",
                        self.func.mutability
                    ),
                    *loc,
                    "access of value sent".into()
                ));
            } else {
                self.diagnostics.push(Diagnostic::error(
                    *loc,
                    format!(
                        "function declared '{}' but this expression accesses value sent, which is only allowed for payable functions",
                        self.func.mutability
                    ),
                ));
            }
        }

        self.required_access.increase_to(Access::Value);
    }

    fn write(&mut self, loc: &pt::Loc) {
        if self.declared_access < Access::Write {
            if let Some(modifier_loc) = &self.modifier {
                self.diagnostics.push(Diagnostic::error_with_note(
                    *modifier_loc,
                    format!(
                        "function declared '{}' but modifier writes to state",
                        self.func.mutability
                    ),
                    *loc,
                    "write to state".into(),
                ));
            } else {
                self.diagnostics.push(Diagnostic::error(
                    *loc,
                    format!(
                        "function declared '{}' but this expression writes to state",
                        self.func.mutability
                    ),
                ));
            }
        }

        self.required_access.increase_to(Access::Write);
    }

    fn read(&mut self, loc: &pt::Loc) {
        if self.declared_access < Access::Read {
            if let Some(modifier_loc) = &self.modifier {
                self.diagnostics.push(Diagnostic::error_with_note(
                    *modifier_loc,
                    format!(
                        "function declared '{}' but modifier reads from state",
                        self.func.mutability
                    ),
                    *loc,
                    "read to state".into(),
                ));
            } else {
                self.diagnostics.push(Diagnostic::error(
                    *loc,
                    format!(
                        "function declared '{}' but this expression reads from state",
                        self.func.mutability
                    ),
                ));
            }
        }

        self.required_access.increase_to(Access::Read);
    }
}

fn check_mutability(func: &Function, ns: &Namespace) -> Vec<Diagnostic> {
    if func.is_virtual {
        return Vec::new();
    }

    let mut state = StateCheck {
        diagnostics: Vec::new(),
        declared_access: match func.mutability {
            Mutability::Pure(_) => Access::None,
            Mutability::View(_) => Access::Read,
            Mutability::Nonpayable(_) => Access::Write,
            Mutability::Payable(_) => Access::Value,
        },
        required_access: Access::None,
        func,
        modifier: None,
        ns,
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

                state.modifier = Some(arg.loc());

                recurse_statements(&func.body, ns, &mut state);

                state.modifier = None;
            }
        }
    }

    recurse_statements(&func.body, ns, &mut state);

    if pt::FunctionTy::Function == func.ty && !func.is_accessor {
        if state.required_access == Access::None {
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

        // don't suggest marking payable as view (declared_access == Value)
        if state.required_access == Access::Read && state.declared_access == Access::Write {
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
                if let Some(next) = next {
                    next.recurse(state, read_expression);
                }
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
            Statement::Revert { args, .. } => {
                for arg in args {
                    arg.recurse(state, read_expression);
                }
            }
            Statement::Break(_) | Statement::Continue(_) | Statement::Underscore(_) => (),
            Statement::Assembly(inline_assembly, _) => {
                for function_no in inline_assembly.functions.start..inline_assembly.functions.end {
                    recurse_yul_statements(&ns.yul_functions[function_no].body.statements, state);
                }
                recurse_yul_statements(&inline_assembly.body, state);
            }
        }
    }
}

fn read_expression(expr: &Expression, state: &mut StateCheck) -> bool {
    match expr {
        Expression::PreIncrement { expr, .. }
        | Expression::PreDecrement { expr, .. }
        | Expression::PostIncrement { expr, .. }
        | Expression::PostDecrement { expr, .. } => {
            expr.recurse(state, write_expression);
        }
        Expression::Assign { left, right, .. } => {
            right.recurse(state, read_expression);
            left.recurse(state, write_expression);
        }
        Expression::StorageVariable { loc, .. } => state.read(loc),
        Expression::StorageArrayLength { loc, .. } | Expression::StorageLoad { loc, .. } => {
            state.read(loc)
        }
        Expression::Subscript { loc, array_ty, .. } if array_ty.is_contract_storage() => {
            state.read(loc)
        }
        Expression::Builtin {
            kind: Builtin::FunctionSelector,
            args,
            ..
        } => {
            if let Expression::ExternalFunction { .. } = &args[0] {
                // in the case of `this.func.selector`, the address of this is not used and
                // therefore does not read state. Do not recurse down the `address` field of
                // Expression::ExternalFunction
                return false;
            }
        }
        Expression::Builtin {
            loc,
            kind:
                Builtin::GetAddress
                | Builtin::BlockNumber
                | Builtin::Slot
                | Builtin::Timestamp
                | Builtin::ProgramId
                | Builtin::BlockCoinbase
                | Builtin::BlockDifficulty
                | Builtin::BlockHash
                | Builtin::Sender
                | Builtin::Origin
                | Builtin::Gasleft
                | Builtin::Gasprice
                | Builtin::GasLimit
                | Builtin::MinimumBalance
                | Builtin::Balance
                | Builtin::Accounts,
            ..
        } => state.read(loc),
        Expression::Builtin {
            loc,
            kind: Builtin::PayableSend | Builtin::PayableTransfer | Builtin::SelfDestruct,
            ..
        } => state.write(loc),
        Expression::Builtin {
            loc,
            kind: Builtin::Value,
            ..
        } => {
            // internal/private functions cannot be declared payable, so msg.value is only checked
            // as reading state in private/internal functions in solc.
            if state.func.is_public() {
                state.value(loc)
            } else {
                state.read(loc)
            }
        }
        Expression::Builtin {
            loc,
            kind: Builtin::ArrayPush | Builtin::ArrayPop,
            args,
            ..
        } if args[0].ty().is_contract_storage() => state.write(loc),

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
        Expression::ExternalFunctionCallRaw { loc, ty, .. } => match ty {
            CallTy::Static => state.read(loc),
            CallTy::Delegate | CallTy::Regular => state.write(loc),
        },
        _ => {
            return true;
        }
    }
    false
}

fn write_expression(expr: &Expression, state: &mut StateCheck) -> bool {
    match expr {
        Expression::StructMember {
            loc, expr: array, ..
        }
        | Expression::Subscript { loc, array, .. } => {
            if array.ty().is_contract_storage() {
                state.write(loc);
                return false;
            }
        }
        Expression::Variable { loc, ty, var_no: _ } => {
            if ty.is_contract_storage() && !expr.ty().is_contract_storage() {
                state.write(loc);
                return false;
            }
        }
        Expression::StorageVariable { loc, .. } => {
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
                recurse_yul_statements(&block.statements, state);
            }
            YulStatement::Assignment(_, _, _, value)
            | YulStatement::VariableDeclaration(_, _, _, Some(value)) => {
                value.recurse(state, check_expression_mutability_yul);
            }
            YulStatement::IfBlock(_, _, condition, block) => {
                condition.recurse(state, check_expression_mutability_yul);
                recurse_yul_statements(&block.statements, state);
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
                    recurse_yul_statements(&item.block.statements, state);
                }

                if let Some(block) = default {
                    recurse_yul_statements(&block.statements, state);
                }
            }
            YulStatement::For {
                init_block,
                condition,
                post_block,
                execution_block,
                ..
            } => {
                recurse_yul_statements(&init_block.statements, state);
                condition.recurse(state, check_expression_mutability_yul);
                recurse_yul_statements(&post_block.statements, state);
                recurse_yul_statements(&execution_block.statements, state);
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
