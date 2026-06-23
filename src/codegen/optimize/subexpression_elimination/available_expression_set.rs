// SPDX-License-Identifier: Apache-2.0

use crate::codegen::subexpression_elimination::available_variable::AvailableVariable;
use crate::codegen::subexpression_elimination::common_subexpression_tracker::CommonSubExpressionTracker;
use crate::codegen::subexpression_elimination::AvailableExpression;
use crate::codegen::subexpression_elimination::{
    AvailableExpressionSet, BasicExpression, ExpressionType, NodeId,
};
use crate::codegen::Expression;
use crate::sema::ast::StringLocation;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

impl<'a> AvailableExpressionSet<'a> {
    /// Deep clone a set
    pub fn deep_clone(&self) -> AvailableExpressionSet<'a> {
        let mut new_set = AvailableExpressionSet {
            expression_memory: HashMap::default(),
            expr_map: self.expr_map.clone(),
            mapped_variable: self.mapped_variable.clone(),
        };

        for (key, value) in &self.expression_memory {
            new_set.expression_memory.insert(
                *key,
                Rc::new(RefCell::new(BasicExpression {
                    expr_type: value.borrow().expr_type.clone(),
                    expression_id: value.borrow().expression_id,
                    children: HashMap::default(),
                    available_variable: value.borrow().available_variable.clone(),
                    block: value.borrow().block,
                    parent_block: value.borrow().parent_block,
                    reference: value.borrow().reference,
                })),
            );
        }

        for (key, value) in &self.expression_memory {
            let node = new_set.expression_memory.get(key).unwrap();
            for child_id in value.borrow().children.keys() {
                node.borrow_mut().children.insert(
                    *child_id,
                    Rc::clone(new_set.expression_memory.get(child_id).unwrap()),
                );
            }
        }

        new_set
    }

    /// Checks if an expression is available on both sets
    fn check_intersection(
        key: &ExpressionType,
        value: NodeId,
        set_2: &AvailableExpressionSet,
    ) -> bool {
        // Basic case: the expression is available only available on one set
        if !set_2.expr_map.contains_key(key) {
            return false;
        }

        // If the expression is a variable, we must ensure that it points to the same node
        if matches!(key, ExpressionType::Variable(_)) {
            return value == set_2.expr_map[key];
        }

        true
    }

    /// When we exit two blocks, we must intersect their set of available expressions
    pub fn intersect_sets(
        &mut self,
        set_2: &AvailableExpressionSet,
        cst: &CommonSubExpressionTracker,
    ) {
        self.expr_map
            .retain(|key, value| AvailableExpressionSet::check_intersection(key, *value, set_2));

        let mut to_maintain: HashSet<usize> = HashSet::new();

        // Check if an expression is available on both sets, but has a different global id
        for (key, node_id) in &self.expr_map {
            if !set_2.expression_memory.contains_key(node_id) {
                to_maintain.insert(*node_id);
                let node_1 = &mut *self.expression_memory[node_id].borrow_mut();
                node_1.children.clear();
                let node_2_id = set_2.expr_map.get(key).unwrap();

                // Find the common ancestor of both blocks. The deepest block after which there are
                // multiple paths to both blocks.
                node_1.parent_block = cst.find_parent_block(
                    node_1.block,
                    set_2.expression_memory[node_2_id].borrow().block,
                    node_1.reference,
                );
                if let (Some(var_id_1), Some(var_id_2)) = (
                    set_2.expression_memory[node_2_id]
                        .borrow()
                        .available_variable
                        .get_var_number(),
                    node_1.available_variable.get_var_number(),
                ) {
                    if var_id_1 != var_id_2 {
                        node_1.available_variable = AvailableVariable::Invalidated;
                    }
                } else if set_2.expression_memory[node_2_id]
                    .borrow()
                    .available_variable
                    .is_invalid()
                {
                    node_1.available_variable = AvailableVariable::Invalidated;
                }
            }
        }

        self.expression_memory.retain(|key, _| {
            set_2.expression_memory.contains_key(key) || to_maintain.contains(key)
        });

        for (key, value) in &self.expression_memory {
            if let Some(node) = set_2.expression_memory.get(key) {
                value.borrow_mut().children.retain(|child_id, _| {
                    node.borrow().children.contains_key(child_id) || to_maintain.contains(child_id)
                });
            }
        }
    }

    /// Calculate the union between two sets
    pub fn union_sets(&mut self, set_2: &AvailableExpressionSet<'a>) {
        let mut node_translation: HashMap<NodeId, NodeId> = HashMap::new();
        for (key, node_id) in &set_2.expr_map {
            if let Some(other_id) = self.expr_map.get(key) {
                node_translation.insert(*node_id, *other_id);
            }
        }

        for (key, node_id) in &set_2.expr_map {
            if !self.expr_map.contains_key(key) {
                let new_key = match key {
                    ExpressionType::BinaryOperation(id_1, id_2, op) => {
                        ExpressionType::BinaryOperation(
                            node_translation.get(id_1).cloned().unwrap_or(*id_1),
                            node_translation.get(id_2).cloned().unwrap_or(*id_2),
                            op.clone(),
                        )
                    }
                    ExpressionType::UnaryOperation(id, op) => ExpressionType::UnaryOperation(
                        node_translation.get(id).cloned().unwrap_or(*id),
                        op.clone(),
                    ),
                    _ => key.clone(),
                };
                self.expr_map.insert(new_key, *node_id);
            }
        }

        for (key, expr) in &set_2.expression_memory {
            if !self.expression_memory.contains_key(key) {
                self.expression_memory.insert(*key, expr.clone());
            }
        }
    }

    /// Check if a commutative expression exists in the set
    fn find_commutative(
        &self,
        exp: &Expression,
        left: &Expression,
        right: &Expression,
    ) -> Option<NodeId> {
        let left_id = self.find_expression(left)?;
        let right_id = self.find_expression(right)?;

        let operator = exp.get_ave_operator();

        if let Some(exp_id) = self.expr_map.get(&ExpressionType::BinaryOperation(
            left_id,
            right_id,
            operator.clone(),
        )) {
            Some(*exp_id)
        } else {
            self.expr_map
                .get(&ExpressionType::BinaryOperation(
                    right_id, left_id, operator,
                ))
                .copied()
        }
    }

    /// Add expressions to the common subexpression tracker.
    fn add_to_cst(&self, exp: &Expression, id: NodeId, cst: &mut CommonSubExpressionTracker) {
        let node = &*self.expression_memory.get(&id).unwrap().borrow();
        cst.add_expression(exp, &node.expr_type, node);
    }

    /// Try to fetch the ID of left and right operands.
    fn process_left_right(
        &mut self,
        left: &'a Expression,
        right: &'a Expression,
        ave: &mut AvailableExpression,
        cst: &mut Option<&mut CommonSubExpressionTracker>,
    ) -> Option<(NodeId, NodeId)> {
        let left_id = self.gen_expression(left, ave, cst)?;
        let right_id = self.gen_expression(right, ave, cst)?;

        Some((left_id, right_id))
    }

    /// Add a commutative expression to the set if it is not there yet
    fn process_commutative(
        &mut self,
        exp: &'a Expression,
        left: &'a Expression,
        right: &'a Expression,
        ave: &mut AvailableExpression,
        cst: &mut Option<&mut CommonSubExpressionTracker>,
    ) -> Option<NodeId> {
        let (left_id, right_id) = self.process_left_right(left, right, ave, cst)?;
        Some(ave.add_binary_node(exp, self, left_id, right_id))
    }

    /// Add expression to the graph and check if it is available on a parallel branch.
    pub fn gen_expression(
        &mut self,
        exp: &'a Expression,
        ave: &mut AvailableExpression,
        cst: &mut Option<&mut CommonSubExpressionTracker>,
    ) -> Option<NodeId> {
        let id = self.gen_expression_aux(exp, ave, cst);
        if let Some(id) = id {
            let node = &*self.expression_memory.get(&id).unwrap().borrow();
            if let Some(tracker) = cst.as_mut() {
                tracker.check_availability_on_branches(&node.expr_type, exp);
            }
        }
        id
    }

    /// Add an expression to the graph if it is not there
    pub fn gen_expression_aux(
        &mut self,
        exp: &'a Expression,
        ave: &mut AvailableExpression,
        cst: &mut Option<&mut CommonSubExpressionTracker>,
    ) -> Option<NodeId> {
        if let Some(id) = self.find_expression(exp) {
            if let Some(tracker) = cst.as_mut() {
                self.add_to_cst(exp, id, tracker);
            }
            return Some(id);
        }

        match exp {
            Expression::Variable { .. } | Expression::FunctionArg { .. } => {
                return Some(ave.add_variable_node(exp, self));
            }

            Expression::NumberLiteral { .. }
            | Expression::BoolLiteral { .. }
            | Expression::BytesLiteral { .. } => {
                let key = exp.get_constant_expression_type();

                let exp_id = if let Some(id) = self.expr_map.get(&key) {
                    *id
                } else {
                    ave.add_literal_node(exp, self)
                };

                return Some(exp_id);
            }

            Expression::StringCompare { left, right, .. } => {
                return if let (
                    StringLocation::RunTime(operand_1),
                    StringLocation::RunTime(operand_2),
                ) = (left, right)
                {
                    self.process_commutative(exp, operand_1, operand_2, ave, cst)
                } else {
                    None
                };
            }

            _ => {}
        }

        // Process commutative expressions
        if let Some((left, right)) = exp.get_commutative_operands() {
            return self.process_commutative(exp, left, right, ave, cst);
        }

        // Process non commutative expressions
        if let Some((left, right)) = exp.get_non_commutative_operands() {
            let (left_id, right_id) = self.process_left_right(left, right, ave, cst)?;
            return Some(ave.add_binary_node(exp, self, left_id, right_id));
        }

        // Process unary expressions
        if let Some(operand) = exp.get_unary_operand() {
            let id = self.gen_expression(operand, ave, cst)?;
            return Some(ave.add_unary_node(exp, id, self));
        }

        // Due to reaching definitions limitations, it is not possible to keep track of
        // all operations
        None
    }

    /// Remove from the set all children from a node
    fn kill_child(&mut self, child_node: &Rc<RefCell<BasicExpression>>, parent_id: NodeId) {
        self.kill_recursive(&child_node.borrow(), parent_id);
        child_node.borrow_mut().children.clear();
    }

    /// Recursively remove from the set all the children of a node
    fn kill_recursive(&mut self, basic_exp: &BasicExpression, parent_id: NodeId) {
        for (child_id, node) in &basic_exp.children {
            self.kill_child(node, basic_exp.expression_id);
            self.expression_memory.remove(child_id);
        }

        if let ExpressionType::BinaryOperation(left, right, _) = &basic_exp.expr_type {
            let other_parent = if *left == parent_id { right } else { left };
            // If the graph has a cycle, we may have already borrowed or deleted a parent.
            if let Some(parent_ref) = self.expression_memory.get_mut(other_parent) {
                if let Ok(mut parent) = parent_ref.try_borrow_mut() {
                    parent.children.remove(&basic_exp.expression_id);
                }
            }
        }

        self.expr_map.remove(&basic_exp.expr_type);
    }

    /// This functions indicates that an available node that was once mapped to an existing variable
    /// no longer should be linked to that variable.
    ///
    /// When we have an assignment 'x = a + b', and later we find the usage of 'a + b', we can
    /// replace it by 'x', instead of creating a new cse temporary. Nonetheless, whenever the 'x'
    /// is reassigned, we must indicate that 'x' does not represent 'a + b' anymore, so we would
    /// need a temporary if we were to replace a repeated occurrence of 'a + b'
    pub fn remove_mapped(&mut self, var_no: usize) {
        if let Some(node_id) = self.mapped_variable.remove(&var_no) {
            if let Some(node) = self.expression_memory.get(&node_id) {
                let mut node_mut = node.borrow_mut();
                if node_mut.available_variable.is_available() {
                    node_mut.available_variable = AvailableVariable::Unavailable;
                }
            }
        }
    }

    /// When a reaching definition changes, we remove the variable node and all its descendants from
    /// the graph
    pub fn kill(&mut self, var_no: usize) {
        let key = ExpressionType::Variable(var_no);
        if !self.expr_map.contains_key(&key) {
            return;
        }

        let var_id = self.expr_map[&key];
        let var_node = self.expression_memory[&var_id].clone();
        for (child_id, node) in &var_node.borrow().children {
            self.kill_child(node, var_id);
            self.expression_memory.remove(child_id);
        }
        self.expression_memory.remove(&var_id);
        self.expr_map.remove(&key);
    }

    /// Check if an expression is available
    pub fn find_expression(&self, exp: &Expression) -> Option<NodeId> {
        match exp {
            Expression::FunctionArg { arg_no, .. } => {
                return self
                    .expr_map
                    .get(&ExpressionType::FunctionArg(*arg_no))
                    .copied();
            }

            Expression::Variable { var_no, .. } => {
                return self
                    .expr_map
                    .get(&ExpressionType::Variable(*var_no))
                    .copied();
            }

            //Expression::ConstantVariable(..)
            Expression::NumberLiteral { .. }
            | Expression::BoolLiteral { .. }
            | Expression::BytesLiteral { .. } => {
                let key = exp.get_constant_expression_type();
                return self.expr_map.get(&key).copied();
            }

            Expression::StringCompare { left, right, .. } => {
                if let (StringLocation::RunTime(operand_1), StringLocation::RunTime(operand_2)) =
                    (left, right)
                {
                    return self.find_commutative(exp, operand_1, operand_2);
                }
            }

            _ => {}
        }

        // Commutative expressions
        if let Some((left, right)) = exp.get_commutative_operands() {
            return self.find_commutative(exp, left, right);
        }

        // Non-commutative expressions
        if let Some((left, right)) = exp.get_non_commutative_operands() {
            let left_id = self.find_expression(left)?;
            let right_id = self.find_expression(right)?;

            let operator = exp.get_ave_operator();

            if let Some(exp_id) = self.expr_map.get(&ExpressionType::BinaryOperation(
                left_id, right_id, operator,
            )) {
                return Some(*exp_id);
            }

            return None;
        }

        // Unary expressions
        if let Some(operand) = exp.get_unary_operand() {
            let id = self.find_expression(operand)?;
            let operator = exp.get_ave_operator();

            if let Some(expr_id) = self
                .expr_map
                .get(&ExpressionType::UnaryOperation(id, operator))
            {
                return Some(*expr_id);
            }

            return None;
        }

        None
    }

    /// Regenerate commutative expressions
    fn regenerate_commutative(
        &mut self,
        exp: &'a Expression,
        left: &'a Expression,
        right: &'a Expression,
        ave: &mut AvailableExpression,
        cst: &mut CommonSubExpressionTracker,
    ) -> (Option<NodeId>, Expression) {
        let (left_id, left_exp) = self.regenerate_expression(left, ave, cst);
        let (right_id, right_exp) = self.regenerate_expression(right, ave, cst);
        let rebuilt_expr = exp.rebuild_binary_expression(&left_exp, &right_exp);

        if left_id.is_none() || right_id.is_none() {
            return (None, rebuilt_expr);
        }

        let operator = exp.get_ave_operator();
        let expr_type_1 =
            ExpressionType::BinaryOperation(left_id.unwrap(), right_id.unwrap(), operator.clone());
        let expr_type_2 =
            ExpressionType::BinaryOperation(right_id.unwrap(), left_id.unwrap(), operator);

        let new_expr = if let Some(regen_var) =
            cst.check_variable_available(&expr_type_1, &rebuilt_expr)
        {
            regen_var
        } else if let Some(regen_var) = cst.check_variable_available(&expr_type_2, &rebuilt_expr) {
            regen_var
        } else {
            rebuilt_expr
        };

        let node_id = if let Some(expr_id) = self.expr_map.get(&expr_type_1) {
            *expr_id
        } else if let Some(expr_id) = self.expr_map.get(&expr_type_2) {
            *expr_id
        } else {
            ave.add_binary_node(exp, self, left_id.unwrap(), right_id.unwrap())
        };

        (Some(node_id), new_expr)
    }

    /// Regenerate expressions, i.e. if there is a common subexpression that can be exchanged by
    /// a temporary, we do it here.
    pub fn regenerate_expression(
        &mut self,
        exp: &'a Expression,
        ave: &mut AvailableExpression,
        cst: &mut CommonSubExpressionTracker,
    ) -> (Option<NodeId>, Expression) {
        match exp {
            // Variables, constants and literals will never be substituted
            Expression::FunctionArg { .. }
            | Expression::Variable {  .. }
            //| Expression::ConstantVariable(..)
            | Expression::NumberLiteral {  .. }
            | Expression::BoolLiteral{..}
            | Expression::BytesLiteral{..} => {
                return (self.gen_expression(exp, ave, &mut Some(cst)), exp.clone());
            }

            Expression::StringCompare { loc: _, left, right }
            => {
                if let (StringLocation::RunTime(operand_1), StringLocation::RunTime(operand_2)) =
                    (left, right)
                {
                    return self.regenerate_commutative(exp, operand_1, operand_2, ave, cst);
                }

                return (None, exp.clone());
            }

            _ => {}
        }

        // Commutative expressions
        if let Some((left, right)) = exp.get_commutative_operands() {
            return self.regenerate_commutative(exp, left, right, ave, cst);
        }

        // Non-commutative expressions
        if let Some((left, right)) = exp.get_non_commutative_operands() {
            let (left_id, left_exp) = self.regenerate_expression(left, ave, cst);
            let (right_id, right_exp) = self.regenerate_expression(right, ave, cst);
            let rebuild_expr = exp.rebuild_binary_expression(&left_exp, &right_exp);

            if left_id.is_none() || right_id.is_none() {
                return (None, rebuild_expr);
            }

            let operator = exp.get_ave_operator();
            let expr_type =
                ExpressionType::BinaryOperation(left_id.unwrap(), right_id.unwrap(), operator);

            let new_expr =
                if let Some(regen_expr) = cst.check_variable_available(&expr_type, &rebuild_expr) {
                    regen_expr
                } else {
                    rebuild_expr
                };

            let node_id = if let Some(expr_id) = self.expr_map.get(&expr_type) {
                *expr_id
            } else {
                ave.add_binary_node(exp, self, left_id.unwrap(), right_id.unwrap())
            };

            return (Some(node_id), new_expr);
        }

        // Unary expressions
        if let Some(operand) = exp.get_unary_operand() {
            let (id, regen_expr) = self.regenerate_expression(operand, ave, cst);
            let rebuilt_expr = exp.rebuild_unary_expression(&regen_expr);

            if id.is_none() {
                return (None, rebuilt_expr);
            }

            let operator = exp.get_ave_operator();
            let expr_type = ExpressionType::UnaryOperation(id.unwrap(), operator);

            let new_expr =
                if let Some(regen_expr) = cst.check_variable_available(&expr_type, &rebuilt_expr) {
                    regen_expr
                } else {
                    rebuilt_expr
                };

            let node_id = if let Some(expr_id) = self.expr_map.get(&expr_type) {
                *expr_id
            } else {
                ave.add_unary_node(exp, id.unwrap(), self)
            };

            return (Some(node_id), new_expr);
        }

        (None, exp.clone())
    }
}
