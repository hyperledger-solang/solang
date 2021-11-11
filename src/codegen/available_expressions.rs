use crate::codegen::cfg::Instr;
use crate::sema::ast::{Expression, StringLocation};
use num_bigint::BigInt;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

/*
The available expression analysis implemented here build a graph to track expressions. Each
operand and each operation represents a vertex. Edges are directed from operands to an operation.
Let's say we have a+b. 'a', 'b' and 'e1=a+b' are vertexes. Edges are directed from 'a' to 'e1=a+b'
and from 'b' to 'a+b'. If we add now 'a+b-c', we will have two new nodes: 'c' and 'e2=e1-c'.
Edges will connect 'c' to 'e2=e1-c' and 'e1=a+b' to 'e2=e1-c'. Whenever a variable becomes
unavailable (i.e. we kill its definition), we recursively remove the operand node and all its
children operations from the graph.
 */

/// This enum defines operator types for the graph
#[derive(PartialEq, Eq, Hash, Copy, Clone)]
enum Operator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Power,
    BitwiseOr,
    BitwiseAnd,
    BitwiseXor,
    ShiftLeft,
    ShiftRight,
    Or,
    And,
    More,
    Less,
    MoreEqual,
    LessEqual,
    Equal,
    NotEqual,
    StringConcat,
    StringCompare,
    //Unary operations
    Not,
    ZeroExt,
    SignExt,
    Trunc,
    Cast,
    BytesCast,
    UnaryMinus,
    Complement,
}

/// NodeId is the identifier of each vertex of the graph
pub type NodeId = usize;

/// Each BasicExpression is a graph node
#[derive(Clone)]
pub struct BasicExpression {
    expr_type: ExpressionType,
    expression_id: NodeId,
    children: HashMap<NodeId, Rc<RefCell<BasicExpression>>>,
}

/// Type of constant to streamline the use of a hashmap
#[derive(Eq, PartialEq, Hash, Clone)]
pub enum ConstantType {
    Bool(bool),
    Bytes(Vec<u8>),
    Number(BigInt),
    ConstantVariable(Option<usize>, usize),
}

/// The type of expression that a node represents
#[derive(Clone, PartialEq, Hash, Eq)]
enum ExpressionType {
    BinaryOperation(NodeId, NodeId, Operator),
    UnaryOperation(NodeId, Operator),
    Variable(usize),
    FunctionArg(usize),
    Literal(ConstantType),
}

/// Sets contain the available expression at a certain portion of the CFG
#[derive(Default)]
pub struct AvailableExpressionSet {
    // node_no => BasicExpression
    expression_memory: HashMap<NodeId, Rc<RefCell<BasicExpression>>>,
    // Expression => node_id
    expr_map: HashMap<ExpressionType, NodeId>,
}

/// This struct serves only to maintain a global id, in such a way that new nodes will always have
/// a different ID
#[derive(Default)]
pub struct AvailableExpression {
    global_id_counter: NodeId,
}

/// Get the respective Operator from an Expression
fn get_operator_from_expression(exp: &Expression) -> Operator {
    match exp {
        Expression::Add(..) => Operator::Add,
        Expression::Subtract(..) => Operator::Subtract,
        Expression::Multiply(..) => Operator::Multiply,
        Expression::Divide(..) => Operator::Divide,
        Expression::Modulo(..) => Operator::Modulo,
        Expression::Power(..) => Operator::Power,
        Expression::BitwiseOr(..) => Operator::BitwiseOr,
        Expression::BitwiseAnd(..) => Operator::BitwiseAnd,
        Expression::BitwiseXor(..) => Operator::BitwiseXor,
        Expression::ShiftLeft(..) => Operator::ShiftLeft,
        Expression::ShiftRight(..) => Operator::ShiftRight,
        Expression::Or(..) => Operator::Or,
        Expression::And(..) => Operator::And,
        Expression::Not(..) => Operator::Not,
        Expression::ZeroExt(..) => Operator::ZeroExt,
        Expression::SignExt(..) => Operator::SignExt,
        Expression::Trunc(..) => Operator::Trunc,
        Expression::Cast(..) => Operator::Cast,
        Expression::BytesCast(..) => Operator::BytesCast,
        Expression::UnaryMinus(..) => Operator::UnaryMinus,
        Expression::More(..) => Operator::More,
        Expression::Less(..) => Operator::Less,
        Expression::MoreEqual(..) => Operator::MoreEqual,
        Expression::LessEqual(..) => Operator::LessEqual,
        Expression::Equal(..) => Operator::Equal,
        Expression::NotEqual(..) => Operator::NotEqual,
        Expression::Complement(..) => Operator::Complement,
        Expression::StringCompare(..) => Operator::StringCompare,
        Expression::StringConcat(..) => Operator::StringConcat,
        _ => {
            unreachable!("Expression does not represent an operator.")
        }
    }
}

impl AvailableExpression {
    /// Add a node to represent a literal
    pub fn add_literal_node(
        &mut self,
        expr: &Expression,
        expr_set: &mut AvailableExpressionSet,
    ) -> NodeId {
        let expr_type = match expr {
            Expression::BoolLiteral(_, value) => {
                ExpressionType::Literal(ConstantType::Bool(*value))
            }

            Expression::NumberLiteral(_, _, value) => {
                ExpressionType::Literal(ConstantType::Number(value.clone()))
            }

            Expression::BytesLiteral(_, _, value) => {
                ExpressionType::Literal(ConstantType::Bytes(value.clone()))
            }

            Expression::ConstantVariable(_, _, contract_no, var_no) => {
                ExpressionType::Literal(ConstantType::ConstantVariable(*contract_no, *var_no))
            }

            _ => unreachable!("This expression is not a literal or a constant variable"),
        };

        expr_set.expression_memory.insert(
            self.global_id_counter,
            Rc::new(RefCell::new(BasicExpression {
                expr_type: expr_type.clone(),
                expression_id: self.global_id_counter,
                children: Default::default(),
            })),
        );

        expr_set.expr_map.insert(expr_type, self.global_id_counter);
        self.global_id_counter += 1;

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

        expr_set.expression_memory.insert(
            self.global_id_counter,
            Rc::new(RefCell::new(BasicExpression {
                expr_type: expr_type.clone(),
                expression_id: self.global_id_counter,
                children: Default::default(),
            })),
        );

        expr_set.expr_map.insert(expr_type, self.global_id_counter);
        self.global_id_counter += 1;

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
        let operation = get_operator_from_expression(exp);
        let new_node = Rc::new(RefCell::new(BasicExpression {
            expr_type: ExpressionType::BinaryOperation(left, right, operation),
            expression_id: self.global_id_counter,
            children: Default::default(),
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
        let operation = get_operator_from_expression(exp);
        let new_node = Rc::new(RefCell::new(BasicExpression {
            expr_type: ExpressionType::UnaryOperation(parent, operation),
            expression_id: self.global_id_counter,
            children: Default::default(),
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
}

impl AvailableExpressionSet {
    /// Check if a commutative expression exists in the set
    fn check_commutative(
        &self,
        exp: &Expression,
        left: &Expression,
        right: &Expression,
    ) -> Option<NodeId> {
        let left_id = self.find_expression(left)?;
        let right_id = self.find_expression(right)?;

        let operator = get_operator_from_expression(exp);

        if let Some(exp_id) = self.expr_map.get(&ExpressionType::BinaryOperation(
            left_id, right_id, operator,
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

    /// Add a commutative expression to the set if it is not there yet
    fn process_commutative(
        &mut self,
        exp: &Expression,
        left: &Expression,
        right: &Expression,
        ave: &mut AvailableExpression,
    ) -> Option<NodeId> {
        let left_id = self.gen_expression(left, ave)?;
        let right_id = self.gen_expression(right, ave)?;

        let operator = get_operator_from_expression(exp);

        if let Some(exp_id) = self.expr_map.get(&ExpressionType::BinaryOperation(
            left_id, right_id, operator,
        )) {
            return Some(*exp_id);
        } else if let Some(exp_id) = self.expr_map.get(&ExpressionType::BinaryOperation(
            right_id, left_id, operator,
        )) {
            return Some(*exp_id);
        }

        Some(ave.add_binary_node(exp, self, left_id, right_id))
    }

    /// Get the hashmap key for a constant variable or a literal
    fn constant_key(exp: &Expression) -> ConstantType {
        match exp {
            Expression::ConstantVariable(_, _, contract_no, var_no) => {
                ConstantType::ConstantVariable(*contract_no, *var_no)
            }

            Expression::BytesLiteral(_, _, value) => ConstantType::Bytes(value.clone()),

            Expression::BoolLiteral(_, value) => ConstantType::Bool(*value),

            Expression::NumberLiteral(_, _, value) => ConstantType::Number(value.clone()),

            _ => unreachable!("Not a constant"),
        }
    }

    fn add_variable_or_arg(
        &mut self,
        exp: &Expression,
        expr_type: &ExpressionType,
        ave: &mut AvailableExpression,
    ) -> NodeId {
        if let Some(id) = self.expr_map.get(expr_type) {
            *id
        } else {
            ave.add_variable_node(exp, self)
        }
    }

    /// Add an expression to the graph if it does not exists there
    fn gen_expression(
        &mut self,
        exp: &Expression,
        ave: &mut AvailableExpression,
    ) -> Option<NodeId> {
        match exp {
            Expression::FunctionArg(_, _, pos) => {
                Some(self.add_variable_or_arg(exp, &ExpressionType::FunctionArg(*pos), ave))
            }

            Expression::Variable(_, _, pos) => {
                Some(self.add_variable_or_arg(exp, &ExpressionType::Variable(*pos), ave))
            }

            Expression::ConstantVariable(..)
            | Expression::NumberLiteral(..)
            | Expression::BoolLiteral(..)
            | Expression::BytesLiteral(..) => {
                let key = AvailableExpressionSet::constant_key(exp);

                let exp_id = if let Some(id) = self.expr_map.get(&ExpressionType::Literal(key)) {
                    *id
                } else {
                    ave.add_literal_node(exp, self)
                };

                Some(exp_id)
            }

            // These operations are commutative
            Expression::Add(_, _, _, left, right)
            | Expression::Multiply(_, _, _, left, right)
            | Expression::BitwiseOr(_, _, left, right)
            | Expression::BitwiseAnd(_, _, left, right)
            | Expression::BitwiseXor(_, _, left, right)
            | Expression::Or(_, left, right)
            | Expression::And(_, left, right)
            | Expression::Equal(_, left, right)
            | Expression::NotEqual(_, left, right) => {
                self.process_commutative(exp, left, right, ave)
            }

            // These operations are not commutative
            Expression::Subtract(_, _, _, left, right)
            | Expression::Divide(_, _, left, right)
            | Expression::Modulo(_, _, left, right)
            | Expression::Power(_, _, _, left, right)
            | Expression::ShiftLeft(_, _, left, right)
            | Expression::ShiftRight(_, _, left, right, _)
            | Expression::More(_, right, left)
            | Expression::Less(_, right, left)
            | Expression::MoreEqual(_, right, left) => {
                let left_id = self.gen_expression(left, ave)?;
                let right_id = self.gen_expression(right, ave)?;

                let operator = get_operator_from_expression(exp);

                if let Some(exp_id) = self.expr_map.get(&ExpressionType::BinaryOperation(
                    left_id, right_id, operator,
                )) {
                    return Some(*exp_id);
                }

                Some(ave.add_binary_node(exp, self, left_id, right_id))
            }

            // Unary operations
            Expression::ZeroExt(_, _, operand)
            | Expression::SignExt(_, _, operand)
            | Expression::Trunc(_, _, operand)
            | Expression::Cast(_, _, operand)
            | Expression::BytesCast(_, _, _, operand)
            | Expression::Not(_, operand)
            | Expression::Complement(_, _, operand)
            | Expression::UnaryMinus(_, _, operand) => {
                let id = self.gen_expression(operand, ave)?;

                let operator = get_operator_from_expression(exp);
                if let Some(expr_id) = self
                    .expr_map
                    .get(&ExpressionType::UnaryOperation(id, operator))
                {
                    return Some(*expr_id);
                }

                Some(ave.add_unary_node(exp, id, self))
            }

            Expression::StringCompare(_, left, right)
            | Expression::StringConcat(_, _, left, right) => {
                if let (StringLocation::RunTime(operand_1), StringLocation::RunTime(operand_2)) =
                    (left, right)
                {
                    return self.process_commutative(exp, operand_1, operand_2, ave);
                }

                None
            }

            // Due to reaching definitions limitations, it is not possible to keep track of
            // the following operations
            Expression::StorageVariable(..)
            | Expression::Load(..)
            | Expression::StorageLoad(..)
            | Expression::Subscript(..)
            | Expression::DynamicArraySubscript(..)
            | Expression::InternalFunction { .. }
            | Expression::ExternalFunction { .. }
            | Expression::InternalFunctionCall { .. }
            | Expression::ExternalFunctionCall { .. }
            | Expression::ExternalFunctionCallRaw { .. } => None,

            _ => None,
        }
    }

    /// Remove from the set all children from a node
    fn kill_child(&mut self, child_node: &Rc<RefCell<BasicExpression>>, parent_id: &NodeId) {
        self.kill_recursive(&*child_node.borrow(), parent_id);
        child_node.borrow_mut().children.clear();
    }

    /// Recursively remove from the set all the children of a node
    fn kill_recursive(&mut self, basic_exp: &BasicExpression, parent_id: &NodeId) {
        for (child_id, node) in &basic_exp.children {
            self.kill_child(node, &basic_exp.expression_id);
            self.expression_memory.remove(child_id);
        }

        if let ExpressionType::BinaryOperation(left, right, _) = &basic_exp.expr_type {
            let other_parent = if *left == *parent_id { right } else { left };
            self.expression_memory
                .get_mut(other_parent)
                .unwrap()
                .borrow_mut()
                .children
                .remove(&basic_exp.expression_id);
        }

        self.expr_map.remove(&basic_exp.expr_type);
    }

    /// When a reaching definition change, we remove the variable node and all its descendants from
    /// the graph
    pub fn kill(&mut self, var_no: usize) {
        let key = ExpressionType::Variable(var_no);
        if !self.expr_map.contains_key(&key) {
            return;
        }

        let var_id = self.expr_map[&key];
        let var_node = self.expression_memory[&var_id].clone();
        for (child_id, node) in &var_node.borrow_mut().children {
            self.kill_child(node, &var_id);
            self.expression_memory.remove(child_id);
        }
        self.expression_memory.remove(&var_id);
        self.expr_map.remove(&key);
    }

    /// Check if we can add the expressions of an instruction to the graph
    pub fn process_instruction(&mut self, instr: &Instr, ave: &mut AvailableExpression) {
        match instr {
            Instr::BranchCond { cond: expr, .. }
            | Instr::Store { dest: expr, .. }
            | Instr::LoadStorage { storage: expr, .. }
            | Instr::ClearStorage { storage: expr, .. }
            | Instr::Print { expr }
            | Instr::AssertFailure { expr: Some(expr) }
            | Instr::PopStorage { storage: expr, .. }
            | Instr::AbiDecode { data: expr, .. }
            | Instr::SelfDestruct { recipient: expr }
            | Instr::Set { expr, .. } => {
                let _ = self.gen_expression(expr, ave);
            }

            Instr::PushMemory { value: expr, .. } => {
                let _ = self.gen_expression(expr, ave);
            }

            Instr::SetStorage { value, storage, .. }
            | Instr::PushStorage { value, storage, .. } => {
                let _ = self.gen_expression(value, ave);
                let _ = self.gen_expression(storage, ave);
            }

            Instr::SetStorageBytes {
                value,
                storage,
                offset,
            } => {
                let _ = self.gen_expression(value, ave);
                let _ = self.gen_expression(storage, ave);
                let _ = self.gen_expression(offset, ave);
            }

            Instr::Return { value: exprs } | Instr::Call { args: exprs, .. } => {
                for expr in exprs {
                    let _ = self.gen_expression(expr, ave);
                }
            }

            Instr::Constructor {
                args,
                value,
                gas,
                salt,
                space,
                ..
            } => {
                for arg in args {
                    let _ = self.gen_expression(arg, ave);
                }
                if let Some(expr) = value {
                    let _ = self.gen_expression(expr, ave);
                }

                let _ = self.gen_expression(gas, ave);

                if let Some(expr) = salt {
                    let _ = self.gen_expression(expr, ave);
                }

                if let Some(expr) = space {
                    let _ = self.gen_expression(expr, ave);
                }
            }

            Instr::ExternalCall {
                address,
                payload,
                value,
                gas,
                ..
            } => {
                if let Some(expr) = address {
                    let _ = self.gen_expression(expr, ave);
                }
                let _ = self.gen_expression(payload, ave);
                let _ = self.gen_expression(value, ave);
                let _ = self.gen_expression(gas, ave);
            }

            Instr::ValueTransfer { address, value, .. } => {
                let _ = self.gen_expression(address, ave);
                let _ = self.gen_expression(value, ave);
            }

            Instr::EmitEvent { data, topics, .. } => {
                for expr in data {
                    let _ = self.gen_expression(expr, ave);
                }

                for expr in topics {
                    let _ = self.gen_expression(expr, ave);
                }
            }

            Instr::AssertFailure { expr: None }
            | Instr::Unreachable
            | Instr::Nop
            | Instr::Branch { .. }
            | Instr::PopMemory { .. } => {}
        }
    }

    fn check_intersection(
        key: &ExpressionType,
        value: &NodeId,
        set_2: &AvailableExpressionSet,
    ) -> bool {
        if !set_2.expr_map.contains_key(key) {
            return false;
        }

        if matches!(key, ExpressionType::Variable(_)) {
            return *value == set_2.expr_map[key];
        }

        true
    }

    /// When we exit two blocks, we must intersect their set of available expressions
    pub fn intersect_sets(&mut self, set_2: &AvailableExpressionSet) {
        self.expr_map
            .retain(|key, value| AvailableExpressionSet::check_intersection(key, value, set_2));

        let mut to_maintain: HashSet<usize> = HashSet::new();

        // Check if an expression is available on both sets, but has a different global id
        for node_id in self.expr_map.values() {
            if !set_2.expression_memory.contains_key(node_id) {
                to_maintain.insert(*node_id);
                self.expression_memory[node_id]
                    .borrow_mut()
                    .children
                    .clear();
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

    fn check_variable_or_arg(&self, expr_type: &ExpressionType) -> Option<NodeId> {
        self.expr_map.get(expr_type).copied()
    }

    /// Check if an expression is available
    pub fn find_expression(&self, exp: &Expression) -> Option<NodeId> {
        match exp {
            Expression::FunctionArg(_, _, pos) => {
                self.check_variable_or_arg(&ExpressionType::FunctionArg(*pos))
            }

            Expression::Variable(_, _, pos) => {
                self.check_variable_or_arg(&ExpressionType::Variable(*pos))
            }

            Expression::ConstantVariable(..)
            | Expression::NumberLiteral(..)
            | Expression::BoolLiteral(..)
            | Expression::BytesLiteral(..) => {
                let key = AvailableExpressionSet::constant_key(exp);
                self.expr_map.get(&ExpressionType::Literal(key)).copied()
            }

            Expression::Add(_, _, _, left, right)
            | Expression::Multiply(_, _, _, left, right)
            | Expression::BitwiseOr(_, _, left, right)
            | Expression::BitwiseAnd(_, _, left, right)
            | Expression::BitwiseXor(_, _, left, right)
            | Expression::Or(_, left, right)
            | Expression::And(_, left, right)
            | Expression::Equal(_, left, right)
            | Expression::NotEqual(_, left, right) => self.check_commutative(exp, left, right),

            // These operations are not commutative
            Expression::Subtract(_, _, _, left, right)
            | Expression::Divide(_, _, left, right)
            | Expression::Modulo(_, _, left, right)
            | Expression::Power(_, _, _, left, right)
            | Expression::ShiftLeft(_, _, left, right)
            | Expression::ShiftRight(_, _, left, right, _)
            | Expression::More(_, right, left)
            | Expression::Less(_, right, left)
            | Expression::MoreEqual(_, right, left) => {
                let left_id = self.find_expression(left)?;
                let right_id = self.find_expression(right)?;

                let operator = get_operator_from_expression(exp);

                if let Some(exp_id) = self.expr_map.get(&ExpressionType::BinaryOperation(
                    left_id, right_id, operator,
                )) {
                    return Some(*exp_id);
                }

                None
            }

            Expression::ZeroExt(_, _, operand)
            | Expression::SignExt(_, _, operand)
            | Expression::Trunc(_, _, operand)
            | Expression::Cast(_, _, operand)
            | Expression::BytesCast(_, _, _, operand)
            | Expression::Not(_, operand)
            | Expression::Complement(_, _, operand)
            | Expression::UnaryMinus(_, _, operand) => {
                let id = self.find_expression(operand)?;

                let operator = get_operator_from_expression(exp);
                if let Some(expr_id) = self
                    .expr_map
                    .get(&ExpressionType::UnaryOperation(id, operator))
                {
                    return Some(*expr_id);
                }

                None
            }

            Expression::StringCompare(_, left, right)
            | Expression::StringConcat(_, _, left, right) => {
                if let (StringLocation::RunTime(operand_1), StringLocation::RunTime(operand_2)) =
                    (left, right)
                {
                    return self.check_commutative(exp, operand_1, operand_2);
                }

                None
            }

            _ => None,
        }
    }
}

impl Clone for AvailableExpressionSet {
    /// Clone a set
    fn clone(&self) -> AvailableExpressionSet {
        let mut new_set = AvailableExpressionSet {
            expression_memory: HashMap::default(),
            expr_map: self.expr_map.clone(),
        };

        for (key, value) in &self.expression_memory {
            new_set.expression_memory.insert(
                *key,
                Rc::new(RefCell::new(BasicExpression {
                    expr_type: value.borrow().expr_type.clone(),
                    expression_id: value.borrow().expression_id,
                    children: HashMap::default(),
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
}
