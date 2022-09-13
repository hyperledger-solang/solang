// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::{BasicBlock, ControlFlowGraph, Instr, InstrOrigin};
use crate::codegen::reaching_definitions::block_edges;
use crate::codegen::subexpression_elimination::available_variable::AvailableVariable;
use crate::codegen::subexpression_elimination::common_subexpression_tracker::CommonSubExpressionTracker;
use crate::codegen::subexpression_elimination::operator::Operator;
use crate::sema::ast::Namespace;
use num_bigint::BigInt;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::rc::Rc;

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

/// Each BasicExpression is a graph node
#[derive(Clone)]
pub struct BasicExpression {
    expr_type: ExpressionType,
    expression_id: NodeId,
    children: HashMap<NodeId, Rc<RefCell<BasicExpression>>>,
    pub available_variable: AvailableVariable,
    pub block: usize,
    pub parent_block: usize,
    pub on_parent_block: bool,
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
#[derive(Default)]
pub struct AvailableExpressionSet {
    // node_no => BasicExpression
    expression_memory: HashMap<NodeId, Rc<RefCell<BasicExpression>>>,
    // Expression => node_id
    expr_map: HashMap<ExpressionType, NodeId>,
    parent_block_no: usize,
}

/// Performs common subexpression elimination
pub fn common_sub_expression_elimination(cfg: &mut ControlFlowGraph, ns: &mut Namespace) {
    let mut ave = AvailableExpression::default();
    let mut cst = CommonSubExpressionTracker::default();

    let mut sets: HashMap<usize, AvailableExpressionSet> = HashMap::new();
    let (visiting_order, dag) = find_visiting_order(cfg);
    cst.set_dag(dag);
    sets.insert(0, AvailableExpressionSet::default());

    // First pass: identify common subexpressions using available expressions analysis
    for (block_no, cycle) in &visiting_order {
        let cur_block = &cfg.blocks[*block_no];
        ave.set_cur_block(*block_no);
        cst.set_cur_block(*block_no);
        let mut cur_set = sets.remove(block_no).unwrap();
        kill_loop_variables(cur_block, &mut cur_set, *cycle);
        for (_, instr) in cur_block.instr.iter() {
            cur_set.process_instruction(instr, &mut ave, &mut cst);
        }

        add_neighbor_blocks(cur_block, &cur_set, block_no, &mut sets, &cst);
    }

    cst.create_variables(ns, cfg);
    sets.clear();

    let mut ave = AvailableExpression::default();
    sets.insert(0, AvailableExpressionSet::default());

    // Second pass: eliminate common subexpressions
    for (block_no, cycle) in &visiting_order {
        let mut cur_set = sets.remove(block_no).unwrap();
        let mut cur_block = &mut cfg.blocks[*block_no];
        ave.set_cur_block(*block_no);
        cst.set_cur_block(*block_no);
        let mut new_instructions: Vec<(InstrOrigin, Instr)> = Vec::new();
        kill_loop_variables(cur_block, &mut cur_set, *cycle);
        for (origin, instr) in cur_block.instr.iter() {
            let instr = cur_set.regenerate_instruction(instr, &mut ave, &mut cst);
            cst.add_new_instructions(&mut new_instructions);
            new_instructions.push((origin.clone(), instr));
        }

        cur_block.instr = new_instructions;
        add_neighbor_blocks(cur_block, &cur_set, block_no, &mut sets, &cst);
    }

    cst.add_parent_block_instructions(cfg);
}

/// Add neighbor block to the hashset of Available expressions to be processed
fn add_neighbor_blocks(
    cur_block: &BasicBlock,
    cur_set: &AvailableExpressionSet,
    block_no: &usize,
    sets: &mut HashMap<usize, AvailableExpressionSet>,
    cst: &CommonSubExpressionTracker,
) {
    for edge in block_edges(cur_block) {
        if let Some(set) = sets.get_mut(&edge) {
            set.intersect_sets(cur_set, cst);
        } else {
            sets.insert(edge, cur_set.clone_for_parent_block(*block_no));
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
        cur_set.kill(*var_no);
    }
}

/// Find the correct visiting order for the CFG traversal, using topological sorting. The visiting
/// order should be the same as the execution order. This function also returns a DAG for the
/// execution graph. This helps us find the lowest common ancestor later.
fn find_visiting_order(cfg: &ControlFlowGraph) -> (Vec<(usize, bool)>, Vec<Vec<usize>>) {
    let mut order: Vec<(usize, bool)> = Vec::with_capacity(cfg.blocks.len());
    let mut visited: HashSet<usize> = HashSet::new();
    let mut stack: HashSet<usize> = HashSet::new();
    let mut has_cycle: Vec<bool> = vec![false; cfg.blocks.len()];
    let mut degrees: Vec<i32> = vec![0; cfg.blocks.len()];
    let mut dag: Vec<Vec<usize>> = Vec::new();
    dag.resize(cfg.blocks.len(), vec![]);

    cfg_dfs(
        0,
        cfg,
        &mut visited,
        &mut stack,
        &mut degrees,
        &mut has_cycle,
        &mut dag,
    );

    let mut queue: VecDeque<usize> = VecDeque::new();
    queue.push_back(0);

    while let Some(block_no) = queue.pop_front() {
        order.push((block_no, has_cycle[block_no]));
        for edge in block_edges(&cfg.blocks[block_no]) {
            degrees[edge] -= 1;
            if degrees[edge] == 0 {
                queue.push_back(edge);
            }
        }
    }

    (order, dag)
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

    for edge in block_edges(&cfg.blocks[block_no]) {
        degrees[edge] += 1;
        if cfg_dfs(edge, cfg, visited, stack, degrees, has_cycle, dag) {
            dag[block_no].push(edge);
        }
    }

    stack.remove(&block_no);
    visited.insert(block_no);

    true
}
