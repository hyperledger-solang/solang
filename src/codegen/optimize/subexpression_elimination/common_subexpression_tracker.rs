// SPDX-License-Identifier: Apache-2.0

use crate::codegen::subexpression_elimination::anticipated_expressions::AnticipatedExpressions;
use crate::codegen::subexpression_elimination::{BasicExpression, ExpressionType};
use crate::codegen::{
    vartable::{Storage, Variable},
    ControlFlowGraph, Expression, Instr,
};
use crate::sema::ast::RetrieveType;
use crate::sema::ast::{Namespace, Type};
use solang_parser::pt::OptionalCodeLocation;
use solang_parser::pt::{Identifier, Loc};
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
pub struct CommonSubExpressionTracker<'a> {
    /// This hash map tracks the inserted common subexpressions. The usize is the index into
    /// common_subexpressions vector
    inserted_subexpressions: HashMap<ExpressionType, usize>,
    /// We store common subexpressions in this vector
    common_subexpressions: Vec<CommonSubexpression>,
    /// The cur_block tracks the current block we are currently analysing
    cur_block: usize,
    /// We save here the new instructions we need to add to the current block
    new_cfg_instr: Vec<Instr>,
    /// Here, we store the instruction we must add to blocks different than the one we are
    /// analysing now
    parent_block_instr: Vec<(usize, Instr)>,
    /// Map from variable number to common subexpression
    mapped_variables: HashMap<usize, usize>,
    /// anticipated_expressions saves the ancipated expressions for every block in the CFG
    anticipated_expressions: AnticipatedExpressions<'a>,
}

impl<'a> CommonSubExpressionTracker<'a> {
    pub(super) fn set_anticipated(&mut self, anticipated: AnticipatedExpressions<'a>) {
        self.anticipated_expressions = anticipated;
    }

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
            Expression::FunctionArg { .. }
                | Expression::Variable { .. }
                | Expression::BytesLiteral { .. }
                | Expression::NumberLiteral { .. } //| Expression::ConstantVariable(..)
        ) {
            return;
        }

        if self.inserted_subexpressions.contains_key(expr_type) {
            return;
        }

        self.inserted_subexpressions
            .insert(expr_type.clone(), self.common_subexpressions.len());

        if let Some(var_no) = node.available_variable.get_var_number() {
            // If we encounter an expression like 'x = y+2', we can map 'x' to 'y+2', whenever possible.
            self.mapped_variables
                .insert(var_no, self.common_subexpressions.len());
        }

        self.common_subexpressions.push(CommonSubexpression {
            in_cfg: node.available_variable.is_available(),
            var_no: node.available_variable.get_var_number(),
            var_loc: node.available_variable.loc_opt(),
            instantiated: false,
            var_type: exp.ty(),
            block: node.block,
            on_parent_block: node.parent_block,
        });
    }

    /// Invalidate a mapped variable
    pub fn invalidate_mapped_variable(&mut self, var_no: usize) {
        if let Some(expr_id) = self.mapped_variables.remove(&var_no) {
            self.common_subexpressions[expr_id].var_loc = None;
            self.common_subexpressions[expr_id].in_cfg = false;
            self.common_subexpressions[expr_id].var_no = None;
        }
    }

    /// Create variables in the CFG
    pub fn create_variables(&mut self, ns: &mut Namespace, cfg: &mut ControlFlowGraph) {
        let mut name_cnt: usize = 0;
        for exp in self.common_subexpressions.iter_mut() {
            if exp.var_no.is_none() {
                name_cnt += 1;
                cfg.vars.insert(
                    ns.next_id,
                    Variable {
                        id: Identifier {
                            loc: Loc::Codegen,
                            name: format!("{name_cnt}.cse_temp"),
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

    /// Check if an expression is available on another branch and find the correct block to place it.
    /// We must make sure that all paths to both branches pass through such a block.
    /// eg.
    /// '''
    /// if (condition) {
    ///    x = a + b;
    /// }
    ///
    /// y = a + b;
    /// '''
    ///
    /// This code can be optimized to:
    ///
    /// '''
    /// temp = a + b;
    /// if (condition) {
    ///     x = temp;
    /// }
    /// y = temp;
    /// '''
    ///
    /// This avoids the repeated calculation of 'a+b'
    pub fn check_availability_on_branches(
        &mut self,
        expr_type: &ExpressionType,
        expr: &Expression,
    ) {
        if let Some(expr_id) = self.inserted_subexpressions.get(expr_type) {
            let expr_block = self.common_subexpressions[*expr_id].block;
            let expr_block = self.common_subexpressions[*expr_id]
                .on_parent_block
                .unwrap_or(expr_block);
            let ancestor = self.find_parent_block(self.cur_block, expr_block, expr);
            if let Some(ancestor_no) = ancestor {
                if ancestor_no != expr_block {
                    let common_expression = &mut self.common_subexpressions[*expr_id];
                    // When an expression is going to be evaluated on a block that's different from
                    // the place where we first saw it, it cannot be replaced by an existing variable.
                    common_expression.var_no = None;
                    common_expression.var_loc = None;
                    common_expression.in_cfg = false;
                    common_expression.on_parent_block = Some(ancestor_no);
                }
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
            let new_instr = Instr::Set {
                loc: Loc::Codegen,
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

        Some(Expression::Variable {
            loc: if common_expression.var_loc.is_some() {
                common_expression.var_loc.unwrap()
            } else {
                Loc::Codegen
            },
            ty: common_expression.var_type.clone(),
            var_no: common_expression.var_no.unwrap(),
        })
    }

    /// Add new instructions to the instruction vector
    pub fn add_new_instructions(&mut self, instr_vec: &mut Vec<Instr>) {
        instr_vec.append(&mut self.new_cfg_instr);
    }

    /// If a variable create should be placed in a different block than where it it read, we
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

    /// For common subexpression elimination to work properly, we need to find the common parent of
    /// two blocks. The parent is the deepest block in which every path from the entry block to both
    /// 'block_1' and 'block_2' passes through such a block, provided that the expression is
    /// anticipated there.
    pub fn find_parent_block(
        &self,
        block_1: usize,
        block_2: usize,
        expr: &Expression,
    ) -> Option<usize> {
        // The analysis is done at another data structure to isolate the logic of traversing the
        // CFG from the end to the beginning (backwards).
        self.anticipated_expressions
            .find_ancestor(block_1, block_2, expr)
    }
}
