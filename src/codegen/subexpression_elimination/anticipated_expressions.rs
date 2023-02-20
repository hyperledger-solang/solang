// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::{ControlFlowGraph, Instr};
use crate::codegen::subexpression_elimination::{
    kill_loop_variables, AvailableExpression, AvailableExpressionSet,
};
use crate::codegen::Expression;
use std::collections::HashMap;

/// The AnticipatedExpression struct manages everything related to traversing the CFG backwards, so
/// we can perform an anticipated expression analysis.
///
/// "An expression is anticipated at a point if it is certain to be evaluated along any
/// path before this expression's value is changed (any variables its evaluation depends
/// on are reassigned)."
///
/// The anticipated expression analysis helps us find where a common subexpression can be evaluated.
#[derive(Default, Clone)]
pub(super) struct AnticipatedExpressions<'a> {
    /// The AvailableExpressionSet for each CFG block, when the graph is evaluated in reverse
    reverse_sets: HashMap<usize, AvailableExpressionSet<'a>>,
    /// The CFG represented ad a DAG, but with each edge reversed
    reverse_dag: Vec<Vec<usize>>,
    /// The order in which we must traverse the CFG. It is its topological sort but reversed.
    traversing_order: Vec<(usize, bool)>,
    /// The depth (distance from the entry block) for each one of the CFG blocks.
    depth: Vec<u16>,
}

impl<'a> AnticipatedExpressions<'a> {
    pub(super) fn new(
        dag: &Vec<Vec<usize>>,
        reverse_dag: Vec<Vec<usize>>,
        traversing_order: Vec<(usize, bool)>,
    ) -> AnticipatedExpressions {
        let mut depth: Vec<u16> = vec![u16::MAX; dag.len()];
        AnticipatedExpressions::blocks_depth(dag, 0, 0, &mut depth);
        AnticipatedExpressions {
            reverse_sets: HashMap::new(),
            reverse_dag,
            traversing_order,
            depth,
        }
    }

    /// Calculate the depth of each CFG block, using dfs (depth first search) traversal.
    fn blocks_depth(dag: &Vec<Vec<usize>>, block: usize, level: u16, depth: &mut [u16]) -> u16 {
        if level < depth[block] {
            depth[block] = level;
        } else {
            return level;
        }

        let mut local_level: u16 = u16::MAX;
        for child in &dag[block] {
            local_level = std::cmp::min(
                local_level,
                AnticipatedExpressions::blocks_depth(dag, *child, level + 1, depth),
            );
        }

        local_level
    }

    /// This function calculates the anticipated expressions for each block. The analysis is similar
    /// to available expressions with a few differences:
    ///
    /// 1. The CFG is traversed backwards: from the last executed block to the entry block.
    /// 2. In each block, we traverse instructions from the last to the first.
    /// 3. When a block has multiple children, we must unite all the anticipated expressions from them
    ///    to perform the analysis for this block.
    pub(super) fn calculate_anticipated_expressions<'b: 'a>(
        &mut self,
        instructions: &'b [Vec<Instr>],
        cfg: &ControlFlowGraph,
    ) {
        let mut reverse_ave = AvailableExpression::default();
        // Traverse the CFG according to its reversed topological order
        for (block_no, cycle) in &self.traversing_order {
            reverse_ave.set_cur_block(*block_no);
            let mut cur_set = self.reverse_sets.get(block_no).cloned().unwrap_or_default();
            kill_loop_variables(&cfg.blocks[*block_no], &mut cur_set, *cycle);

            // Iterate over all instructions in reverse
            for instr in instructions[*block_no].iter().rev() {
                cur_set.process_instruction(instr, &mut reverse_ave, &mut None);
            }

            for edge in &self.reverse_dag[*block_no] {
                if let Some(set) = self.reverse_sets.get_mut(edge) {
                    // Instead of intersection two sets as in available expressions,
                    // in anticipated expressions we need to unite them, because the expressions
                    // of all a block's descendants can be anticipated there.
                    set.union_sets(&cur_set);
                } else {
                    self.reverse_sets
                        .insert(*edge, cur_set.clone_for_parent_block(*block_no));
                }
            }
        }
    }

    /// We calculate the flow in the graph considering block_1 and block_2 as sources, and using
    /// the reversed CFG DAG. If any block has a flow that equals the sum of the two sources,
    /// it can be used to calculate a common expressions that exists in both block_1 and block_2.
    /// In the common subexpression elimination context, a block that has a total flow of
    /// flow[block_1]+flow[block_2] means that it is a code path that leads to both block_1 and
    /// block_2.
    ///
    /// When I use the term flow, I am referring to a flow network (https://en.wikipedia.org/wiki/Flow_network).
    /// The flow of each vertex is equally divided between its children, and the flow a vertex
    /// receives is the sum of the flows from its incoming edges.
    pub(super) fn calculate_flow(&self, block_1: usize, block_2: usize) -> Vec<f32> {
        let mut flow: Vec<f32> = vec![0.0; self.reverse_dag.len()];
        flow[block_1] = 1000.0;
        flow[block_2] = 1000.0;

        for (block_no, _) in &self.traversing_order {
            let divided_flow = flow[*block_no] / (self.reverse_dag[*block_no].len() as f32);
            for child in &self.reverse_dag[*block_no] {
                flow[*child] += divided_flow;
            }
        }

        flow
    }

    /// This function find the correct block to place the evaluation of the common subexpression
    /// 'expr', considering flow, depth and its anticipated availability.
    pub(super) fn find_ancestor(
        &self,
        block_1: usize,
        block_2: usize,
        expr: &Expression,
    ) -> Option<usize> {
        if block_1 == block_2 {
            return Some(block_1);
        }

        let flow = self.calculate_flow(block_1, block_2);

        let mut candidate = usize::MAX;

        for (block_no, flow_magnitude) in flow.iter().enumerate() {
            // The condition is the following:
            // 1. We prefer deeper blocks to evaluate the subexpression (depth[block_no] < depth[candidate]).
            //    This is because if we evaluate a subexpression too early, we risk taking a branch
            //    where the subexpression is not even used.
            // 2. The flow_magnitude must be 2000. (2000.0 - *flow_magnitude).abs() deals with
            //    floating point imprecision. We can also set a lower threshold for the comparison.
            //    Ideally, it should be greater than the machine epsilon.
            // 3. The expression must be available at the anticipated expression set for the block
            //    we are analysing.
            if (candidate == usize::MAX || self.depth[block_no] > self.depth[candidate])
                && (2000.0 - *flow_magnitude).abs() < 0.000001
                && self
                    .reverse_sets
                    .get(&block_no)
                    .unwrap()
                    .find_expression(expr)
                    .is_some()
            {
                candidate = block_no;
            }
        }

        if candidate < usize::MAX {
            Some(candidate)
        } else {
            None
        }
    }
}

#[test]
fn test_depth() {
    let dag = vec![
        vec![1, 2], // 0 -> 1, 2
        vec![3, 4], // 1 -> 3, 4
        vec![3, 4], // 2 -> 3, 4
        vec![],     // 3
        vec![],     // 4
    ];
    let mut depth: Vec<u16> = vec![u16::MAX; 5];
    AnticipatedExpressions::blocks_depth(&dag, 0, 0, &mut depth);
    assert_eq!(depth, vec![0, 1, 1, 2, 2]);

    let dag = vec![
        vec![1, 2, 4], // 0 -> 1, 2, 4
        vec![2, 3],    // 1 -> 2, 3
        vec![4],       // 2 -> 4
        vec![],        // 3
        vec![],        // 4
    ];
    let mut depth: Vec<u16> = vec![u16::MAX; 5];
    AnticipatedExpressions::blocks_depth(&dag, 0, 0, &mut depth);
    assert_eq!(depth, vec![0, 1, 1, 2, 1]);

    let dag = vec![
        vec![1, 4], // 0 -> 1, 4
        vec![2, 3], // 1 -> 2, 3
        vec![],     // 2
        vec![5],    // 3 -> 5
        vec![5],    // 4 -> 5
        vec![],     // 5
    ];
    let mut depth: Vec<u16> = vec![u16::MAX; 6];
    AnticipatedExpressions::blocks_depth(&dag, 0, 0, &mut depth);
    assert_eq!(depth, vec![0, 1, 2, 2, 1, 2]);

    let dag = vec![
        vec![1, 6],    // 0 -> 1, 6
        vec![2, 4],    // 1 -> 2, 4
        vec![3, 4],    // 2 -> 3, 4
        vec![],        // 3
        vec![5],       // 4 -> 5
        vec![],        // 5
        vec![4, 7, 8], // 6 -> 4, 7, 8
        vec![5],       // 7 -> 5
        vec![],        // 8
    ];
    let mut depth: Vec<u16> = vec![u16::MAX; 9];
    AnticipatedExpressions::blocks_depth(&dag, 0, 0, &mut depth);
    assert_eq!(depth, vec![0, 1, 2, 3, 2, 3, 1, 2, 2]);

    // Case 5
    let dag = vec![
        vec![1, 3],    // 0 -> 1, 3
        vec![2, 4],    // 1 -> 2, 4
        vec![7, 8, 6], // 2 -> 7, 8, 6
        vec![2, 6],    // 3 -> 2, 6
        vec![7, 5],    // 4 -> 7, 5
        vec![],        // 5
        vec![],        // 6
        vec![],        // 7
        vec![],        // 8
    ];
    let mut depth: Vec<u16> = vec![u16::MAX; 9];
    AnticipatedExpressions::blocks_depth(&dag, 0, 0, &mut depth);
    assert_eq!(depth, vec![0, 1, 2, 1, 2, 3, 2, 3, 3]);

    // Loop case
    let dag = vec![
        vec![1], // 0 -> 1
        vec![2], // 1 -> 2
        vec![3], // 2 -> 3
        vec![1], // 3 -> 1
    ];
    let mut depth: Vec<u16> = vec![u16::MAX; 4];
    AnticipatedExpressions::blocks_depth(&dag, 0, 0, &mut depth);
    assert_eq!(depth, vec![0, 1, 2, 3]);
}
