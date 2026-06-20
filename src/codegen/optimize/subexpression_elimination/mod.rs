// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::{BasicBlock, ControlFlowGraph, Instr};
use crate::codegen::subexpression_elimination::anticipated_expressions::AnticipatedExpressions;
use crate::codegen::subexpression_elimination::available_variable::AvailableVariable;
use crate::codegen::subexpression_elimination::common_subexpression_tracker::CommonSubExpressionTracker;
use crate::codegen::subexpression_elimination::operator::Operator;
use crate::codegen::Expression;
use crate::sema::ast::Namespace;
use num_bigint::BigInt;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::rc::Rc;

mod anticipated_expressions;
mod available_expression;
mod available_expression_set;
mod available_variable;
pub mod common_subexpression_tracker;
mod expression;
mod instruction;
mod operator;
mod tests;

/*
The available expression analysis implemented here builds a graph to track expressions. Each
operand and each operation represents a vertex. Edges are directed from operands to an operation.
Let's say we have a+b. 'a', 'b' and 'e1=a+b' are vertexes. Edges are directed from 'a' to 'e1=a+b'
and from 'b' to 'a+b'. If we add now 'a+b-c', we will have two new nodes: 'c' and 'e2=e1-c'.
Edges will connect 'c' to 'e2=e1-c' and 'e1=a+b' to 'e2=e1-c'. Whenever a variable becomes
unavailable (i.e. we kill its definition), we recursively remove the operand node and all its
children operations from the graph.

For the common subexpression elimination, whenever we are trying to add an expression to the graph
that is already available, we track it with the CommonSubexpressionTracker. During another pass on
the CFG, we check if we are adding an expression that is tracked. If so, we can regenerate the
ast::Expression using the new temporary variable.
 */

/// NodeId is the identifier of each vertex of the graph
pub type NodeId = usize;

/// This struct serves only to maintain a global id, in such a way that new nodes will always have
/// a different ID
#[derive(Default)]
pub struct AvailableExpression {
    global_id_counter: NodeId,
    cur_block: usize,
}

/// Each BasicExpression is a graph node that tracks a real codegen::Expression
#[derive(Clone)]
pub struct BasicExpression<'a> {
    /// The expression type for this node
    expr_type: ExpressionType,
    /// The node global id
    expression_id: NodeId,
    /// This map tracks all the node's children
    children: HashMap<NodeId, Rc<RefCell<BasicExpression<'a>>>>,
    /// Reference points to the real codegen::Expression this node represents
    pub reference: &'a Expression,
    /// Available_variable tells us if a CFG variable is available for this node.
    /// E.g. if 'x=a+b' is evaluated, 'a+b' is already assigned to a variable in the CFG, so it
    /// does not need a temporary in case it happens to be a common subexpression.
    pub available_variable: AvailableVariable,
    /// Block is the CFG block where the expression was first seen.
    pub block: usize,
    /// When parent_block is set, the expression should be evaluated at the parent_block instead of
    /// block (the parameter right above this one).
    pub parent_block: Option<usize>,
}

/// Type of constant to streamline the use of a hashmap
#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub enum ConstantType {
    Bool(bool),
    Bytes(Vec<u8>),
    Number(BigInt),
}

/// The type of expression that a node represents
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum ExpressionType {
    BinaryOperation(NodeId, NodeId, Operator),
    UnaryOperation(NodeId, Operator),
    Variable(usize),
    FunctionArg(usize),
    Literal(ConstantType),
}

/// Sets contain the available expression at a certain portion of the CFG
#[derive(Default, Clone)]
pub struct AvailableExpressionSet<'a> {
    // node_no => BasicExpression
    expression_memory: HashMap<NodeId, Rc<RefCell<BasicExpression<'a>>>>,
    // Expression => node_id
    expr_map: HashMap<ExpressionType, NodeId>,
    mapped_variable: HashMap<usize, NodeId>,
}

/// This struct serves to be the return of function 'find_visiting_order', which helps finding all
/// the CFG representations for the analysis.
struct CfgAsDag {
    visiting_order: Vec<(usize, bool)>,
    reverse_visiting_order: Vec<(usize, bool)>,
    dag: Vec<Vec<usize>>,
    reverse_dag: Vec<Vec<usize>>,
}

/// Performs common subexpression elimination
pub fn common_sub_expression_elimination(cfg: &mut ControlFlowGraph, ns: &mut Namespace) {
    // visiting_order: the order in which we should traverse the CFG (this is its topological sorting)
    // dag: The CFG represented as a DAG (direct acyclic graph)
    // reverse_dag: The CFG represented as a DAG, but with all the edges reversed.
    let cfg_as_dag = find_visiting_order(cfg);

    let mut old_instr: Vec<Vec<Instr>> = vec![Vec::new(); cfg.blocks.len()];
    // We need to remove the instructions from the blocks, so we can store references in the
    // available expressions set.
    for (block_no, block) in cfg.blocks.iter_mut().enumerate() {
        std::mem::swap(&mut old_instr[block_no], &mut block.instr);
    }

    // Anticipated expression is an analysis that calculated where in the CFG we can anticipate
    // the evaluation of an expression, provided that none of its constituents variables have
    // been assigned a new value.
    let mut anticipated_expressions = AnticipatedExpressions::new(
        &cfg_as_dag.dag,
        cfg_as_dag.reverse_dag,
        cfg_as_dag.reverse_visiting_order,
    );
    anticipated_expressions.calculate_anticipated_expressions(&old_instr, cfg);

    let mut ave = AvailableExpression::default();
    let mut cst = CommonSubExpressionTracker::default();
    let mut sets: HashMap<usize, AvailableExpressionSet> = HashMap::new();
    sets.insert(0, AvailableExpressionSet::default());
    // The anticipated expression values are part of the common subsexpression tracker, so we
    // can always evaluate the common subexpressions in the correct place.
    cst.set_anticipated(anticipated_expressions);

    // First pass: identify common subexpressions using available expressions analysis
    for (block_no, cycle) in &cfg_as_dag.visiting_order {
        let cur_block = &cfg.blocks[*block_no];
        ave.set_cur_block(*block_no);
        cst.set_cur_block(*block_no);
        let mut cur_set = sets.remove(block_no).unwrap();
        kill_loop_variables(cur_block, &mut cur_set, *cycle);
        for instr in old_instr[*block_no].iter() {
            cur_set.process_instruction(instr, &mut ave, &mut Some(&mut cst));
        }

        add_neighbor_blocks(cur_set, &cfg_as_dag.dag[*block_no], &mut sets, &cst);
    }

    cst.create_variables(ns, cfg);
    sets.clear();

    let mut ave = AvailableExpression::default();
    sets.insert(0, AvailableExpressionSet::default());

    // Second pass: eliminate common subexpressions
    for (block_no, cycle) in &cfg_as_dag.visiting_order {
        let mut cur_set = sets.remove(block_no).unwrap();
        let cur_block = &mut cfg.blocks[*block_no];
        ave.set_cur_block(*block_no);
        cst.set_cur_block(*block_no);
        let mut new_instructions: Vec<Instr> = Vec::new();
        kill_loop_variables(cur_block, &mut cur_set, *cycle);
        for instr in old_instr[*block_no].iter() {
            let instr = cur_set.regenerate_instruction(instr, &mut ave, &mut cst);
            cst.add_new_instructions(&mut new_instructions);
            new_instructions.push(instr);
        }

        cur_block.instr = new_instructions;
        add_neighbor_blocks(cur_set, &cfg_as_dag.dag[*block_no], &mut sets, &cst);
    }

    cst.add_parent_block_instructions(cfg);
}

/// Add neighbor block to the hashset of Available expressions to be processed
fn add_neighbor_blocks<'b>(
    cur_set: AvailableExpressionSet<'b>,
    edges: &[usize],
    sets: &mut HashMap<usize, AvailableExpressionSet<'b>>,
    cst: &CommonSubExpressionTracker,
) {
    for edge in edges {
        if let Some(set) = sets.get_mut(edge) {
            set.intersect_sets(&cur_set, cst);
        } else {
            sets.insert(*edge, cur_set.deep_clone());
        }
    }
}

/// When there is a cycle in the CFG, definitions from a loop can reach the current block. In this
/// case, we must kill previous definitions of the given variable.
fn kill_loop_variables(block: &BasicBlock, cur_set: &mut AvailableExpressionSet, has_cycle: bool) {
    if !has_cycle {
        return;
    }
    for var_no in &block.loop_reaching_variables {
        cur_set.remove_mapped(*var_no);
        cur_set.kill(*var_no);
    }
}

/// Find the correct visiting order for the CFG traversal, using topological sorting. The visiting
/// order should be the same as the execution order. This function also returns a DAG for the
/// execution graph. This helps us find the lowest common ancestor later.
fn find_visiting_order(cfg: &ControlFlowGraph) -> CfgAsDag {
    let mut order: Vec<(usize, bool)> = Vec::with_capacity(cfg.blocks.len());
    let mut visited: HashSet<usize> = HashSet::new();
    let mut stack: HashSet<usize> = HashSet::new();
    let mut has_cycle: Vec<bool> = vec![false; cfg.blocks.len()];
    let mut degrees: Vec<i32> = vec![0; cfg.blocks.len()];
    let mut dag: Vec<Vec<usize>> = Vec::new();
    let mut reverse_dag: Vec<Vec<usize>> = Vec::new();
    dag.resize(cfg.blocks.len(), vec![]);
    reverse_dag.resize(cfg.blocks.len(), vec![]);

    cfg_dfs(
        0,
        cfg,
        &mut visited,
        &mut stack,
        &mut degrees,
        &mut has_cycle,
        &mut dag,
        &mut reverse_dag,
    );

    let mut queue: VecDeque<usize> = VecDeque::new();
    queue.push_back(0);

    while let Some(block_no) = queue.pop_front() {
        order.push((block_no, has_cycle[block_no]));
        for edge in cfg.blocks[block_no].successors() {
            degrees[edge] -= 1;
            if degrees[edge] == 0 {
                queue.push_back(edge);
            }
        }
    }

    let mut reverse_visiting_order = order.clone();
    reverse_visiting_order.reverse();
    CfgAsDag {
        visiting_order: order,
        reverse_visiting_order,
        dag,
        reverse_dag,
    }
}

/// Run DFS (depth first search) in the CFG to find cycles.
fn cfg_dfs(
    block_no: usize,
    cfg: &ControlFlowGraph,
    visited: &mut HashSet<usize>,
    stack: &mut HashSet<usize>,
    degrees: &mut Vec<i32>,
    has_cycle: &mut Vec<bool>,
    dag: &mut Vec<Vec<usize>>,
    reverse_dag: &mut Vec<Vec<usize>>,
) -> bool {
    if visited.contains(&block_no) {
        return true;
    }

    if stack.contains(&block_no) {
        degrees[block_no] -= 1;
        has_cycle[block_no] = true;
        return false;
    }

    stack.insert(block_no);

    for edge in cfg.blocks[block_no].successors() {
        degrees[edge] += 1;
        if cfg_dfs(
            edge,
            cfg,
            visited,
            stack,
            degrees,
            has_cycle,
            dag,
            reverse_dag,
        ) {
            dag[block_no].push(edge);
            reverse_dag[edge].push(block_no);
        }
    }

    stack.remove(&block_no);
    visited.insert(block_no);

    true
}
