use crate::codegen::subexpression_elimination::{
    AvailableExpression, AvailableExpressionSet, AvailableVariable, BasicExpression,
    ExpressionType, NodeId,
};
use crate::sema::ast::Expression;
use std::cell::RefCell;
use std::rc::Rc;

impl AvailableExpression {
    /// Add a node to represent a literal
    pub fn add_literal_node(
        &mut self,
        expr: &Expression,
        expr_set: &mut AvailableExpressionSet,
    ) -> NodeId {
        let expr_type = expr.get_constant_expression_type();

        self.add_node_to_memory(expr_set, expr_type);

        self.global_id_counter - 1
    }

    /// Add a node to represent a variable
    pub fn add_variable_node(
        &mut self,
        expr: &Expression,
        expr_set: &mut AvailableExpressionSet,
    ) -> NodeId {
        let expr_type = match expr {
            Expression::Variable(_, _, pos) => ExpressionType::Variable(*pos),

            Expression::FunctionArg(_, _, pos) => ExpressionType::FunctionArg(*pos),

            _ => unreachable!("This expression is not a variable or a function argument"),
        };

        self.add_node_to_memory(expr_set, expr_type);

        self.global_id_counter - 1
    }

    /// Add a node to represent a binary expression
    pub fn add_binary_node(
        &mut self,
        exp: &Expression,
        expr_set: &mut AvailableExpressionSet,
        left: NodeId,
        right: NodeId,
    ) -> NodeId {
        let operation = exp.get_ave_operator();
        let new_node = Rc::new(RefCell::new(BasicExpression {
            expr_type: ExpressionType::BinaryOperation(left, right, operation),
            expression_id: self.global_id_counter,
            children: Default::default(),
            available_variable: AvailableVariable::Unavailable,
            parent_block: expr_set.parent_block_no,
            on_parent_block: false,
            block: self.cur_block,
        }));
        expr_set
            .expression_memory
            .insert(self.global_id_counter, Rc::clone(&new_node));

        expr_set.expr_map.insert(
            ExpressionType::BinaryOperation(left, right, operation),
            self.global_id_counter,
        );

        expr_set
            .expression_memory
            .get_mut(&left)
            .unwrap()
            .borrow_mut()
            .children
            .insert(self.global_id_counter, Rc::clone(&new_node));
        expr_set
            .expression_memory
            .get_mut(&right)
            .unwrap()
            .borrow_mut()
            .children
            .insert(self.global_id_counter, Rc::clone(&new_node));

        self.global_id_counter += 1;
        self.global_id_counter - 1
    }

    /// Add a node to represent an unary operation
    pub fn add_unary_node(
        &mut self,
        exp: &Expression,
        parent: usize,
        expr_set: &mut AvailableExpressionSet,
    ) -> NodeId {
        let operation = exp.get_ave_operator();
        let new_node = Rc::new(RefCell::new(BasicExpression {
            expr_type: ExpressionType::UnaryOperation(parent, operation),
            expression_id: self.global_id_counter,
            children: Default::default(),
            available_variable: AvailableVariable::Unavailable,
            parent_block: expr_set.parent_block_no,
            on_parent_block: false,
            block: self.cur_block,
        }));

        expr_set
            .expression_memory
            .insert(self.global_id_counter, Rc::clone(&new_node));

        expr_set.expr_map.insert(
            ExpressionType::UnaryOperation(parent, operation),
            self.global_id_counter,
        );
        expr_set
            .expression_memory
            .get_mut(&parent)
            .unwrap()
            .borrow_mut()
            .children
            .insert(self.global_id_counter, Rc::clone(&new_node));

        self.global_id_counter += 1;

        self.global_id_counter - 1
    }

    fn add_node_to_memory(
        &mut self,
        expr_set: &mut AvailableExpressionSet,
        expr_type: ExpressionType,
    ) {
        expr_set.expression_memory.insert(
            self.global_id_counter,
            Rc::new(RefCell::new(BasicExpression {
                expr_type: expr_type.clone(),
                expression_id: self.global_id_counter,
                children: Default::default(),
                available_variable: AvailableVariable::Unavailable,
                parent_block: expr_set.parent_block_no,
                on_parent_block: false,
                block: self.cur_block,
            })),
        );

        expr_set.expr_map.insert(expr_type, self.global_id_counter);
        self.global_id_counter += 1;
    }

    /// Set the current block being processed. We save this number in the BasicExpression node for
    /// each new expression.
    pub fn set_cur_block(&mut self, block_no: usize) {
        self.cur_block = block_no;
    }
}
