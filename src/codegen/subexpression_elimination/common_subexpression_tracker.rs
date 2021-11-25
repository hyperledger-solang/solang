use crate::codegen::subexpression_elimination::{BasicExpression, ExpressionType};
use crate::codegen::{
    vartable::{Storage, Variable},
    ControlFlowGraph, Instr,
};
use crate::parser::pt::{Identifier, Loc};
use crate::sema::ast::{Expression, Namespace, Type};
use std::collections::HashMap;

#[derive(Clone)]
struct CommonSubexpression {
    var_no: Option<usize>,
    var_loc: Option<Loc>,
    var_type: Type,
    instantiated: bool,
    in_cfg: bool,
    block: usize,
    on_parent_block: Option<usize>,
}

#[derive(Default, Clone)]
pub struct CommonSubExpressionTracker {
    inserted_subexpressions: HashMap<ExpressionType, usize>,
    common_subexpressions: Vec<CommonSubexpression>,
    len: usize,
    name_cnt: usize,
    cur_block: usize,
    new_cfg_instr: Vec<Instr>,
    parent_block_instr: Vec<(usize, Instr)>,
}

impl CommonSubExpressionTracker {
    /// Add an expression to the tracker.
    pub fn add_expression(
        &mut self,
        exp: &Expression,
        expr_type: &ExpressionType,
        node: &BasicExpression,
    ) {
        // Variables, Literals and constants shouldn't be added,
        // as we are not supposed to exchange them by temporaries.
        if matches!(
            exp,
            Expression::FunctionArg(..)
                | Expression::Variable(..)
                | Expression::BytesLiteral(..)
                | Expression::NumberLiteral(..)
                | Expression::ConstantVariable(..)
        ) {
            return;
        }

        if self.inserted_subexpressions.contains_key(expr_type) {
            return;
        }

        self.inserted_subexpressions
            .insert(expr_type.clone(), self.len);
        self.len += 1;
        self.common_subexpressions.push(CommonSubexpression {
            in_cfg: node.available_variable.is_available(),
            var_no: node.available_variable.get_var_number(),
            var_loc: node.available_variable.get_var_loc(),
            instantiated: false,
            var_type: exp.ty(),
            block: node.block,
            on_parent_block: if node.on_parent_block {
                Some(node.parent_block)
            } else {
                None
            },
        });
    }

    /// Create variables in the CFG
    pub fn create_variables(&mut self, ns: &mut Namespace, cfg: &mut ControlFlowGraph) {
        for exp in self.common_subexpressions.iter_mut() {
            if exp.var_no.is_none() {
                self.name_cnt += 1;
                cfg.vars.insert(
                    ns.next_id,
                    Variable {
                        id: Identifier {
                            loc: Loc(0, 0, 0),
                            name: format!("{}.cse_temp", self.name_cnt),
                        },
                        ty: exp.var_type.clone(),
                        storage: Storage::Local,
                    },
                );
                exp.instantiated = true;
                exp.var_no = Some(ns.next_id);
                ns.next_id += 1;
            }
        }
    }

    /// Try exchanging an expression by a temporary variable.
    pub fn check_variable_available(
        &mut self,
        expr_type: &ExpressionType,
        exp: &Expression,
    ) -> Option<Expression> {
        let expr_id = self.inserted_subexpressions.get(expr_type)?;
        let common_expression = &mut self.common_subexpressions[*expr_id];
        // If there is a variable available, but it has not ben instantiated yet:
        // e.g.
        // x = a+b;
        // y = a+b;
        // we can exchange 'y = a+b' for y=x, but only after x has been instantiated.
        if !common_expression.instantiated {
            common_expression.instantiated = true;
            return None;
        }

        if !common_expression.in_cfg {
            // If there is an expression available, but not for the current block.
            /*
            if (condition) {
                x = a + b;
            }

            y = a+b;
             */
            // 'a+b' is available, but not for the block that contains the branch.
            if self.cur_block != common_expression.block {
                return None;
            }

            let new_instr = Instr::Set {
                loc: Loc(0, 0, 0),
                res: common_expression.var_no.unwrap(),
                expr: exp.clone(),
            };

            if common_expression.on_parent_block.is_none() {
                self.new_cfg_instr.push(new_instr);
            } else {
                self.parent_block_instr
                    .push((common_expression.on_parent_block.unwrap(), new_instr));
            }

            common_expression.in_cfg = true;
        }

        Some(Expression::Variable(
            if common_expression.var_loc.is_some() {
                common_expression.var_loc.unwrap()
            } else {
                Loc(0, 0, 0)
            },
            common_expression.var_type.clone(),
            common_expression.var_no.unwrap(),
        ))
    }

    /// Add new instructions to the instruction vector
    pub fn add_new_instructions(&mut self, instr_vec: &mut Vec<Instr>) {
        instr_vec.append(&mut self.new_cfg_instr);
    }

    /// If a variable create should be hoisted in a different block than where it it read, we
    /// do it here.
    pub fn add_parent_block_instructions(&self, cfg: &mut ControlFlowGraph) {
        for (block_no, instr) in &self.parent_block_instr {
            let index = cfg.blocks[*block_no].instr.len() - 1;
            cfg.blocks[*block_no].instr.insert(index, instr.to_owned());
        }
    }

    /// Set the current block to the CST. This allows us to track where expressions are available
    /// for substitution.
    pub fn set_cur_block(&mut self, block_no: usize) {
        self.cur_block = block_no;
    }
}
