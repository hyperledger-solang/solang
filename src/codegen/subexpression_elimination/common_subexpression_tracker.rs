use crate::codegen::subexpression_elimination::{BasicExpression, ExpressionType};
use crate::codegen::{
    vartable::{Storage, Variable},
    ControlFlowGraph, Expression, Instr,
};
use crate::parser::pt::OptionalCodeLocation;
use crate::parser::pt::{Identifier, Loc};
use crate::sema::ast::RetrieveType;
use crate::sema::ast::{Namespace, Type};
use bitflags::bitflags;
use std::collections::{HashMap, VecDeque};

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

bitflags! {
  struct Color: u8 {
     const WHITE = 0;
     const BLUE = 2;
     const YELLOW = 4;
     const GREEN = 6;
  }
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
    /// The CFG is a cyclic graph. In order properly find the lowest common block,
    /// we transformed it in a DAG, removing cycles from loops.
    cfg_dag: Vec<Vec<usize>>,
}

impl CommonSubExpressionTracker {
    /// Save the DAG to the CST
    pub fn set_dag(&mut self, dag: Vec<Vec<usize>>) {
        self.cfg_dag = dag;
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
            Expression::FunctionArg(..)
                | Expression::Variable(..)
                | Expression::BytesLiteral(..)
                | Expression::NumberLiteral(..) //| Expression::ConstantVariable(..)
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
            var_loc: node.available_variable.loc(),
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
                            loc: Loc::Codegen,
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
    pub fn check_availability_on_branches(&mut self, expr_type: &ExpressionType) {
        if let Some(expr_id) = self.inserted_subexpressions.get(expr_type) {
            let expr_block = self.common_subexpressions[*expr_id].block;
            let expr_block = self.common_subexpressions[*expr_id]
                .on_parent_block
                .unwrap_or(expr_block);
            let ancestor = self.find_parent_block(self.cur_block, expr_block);
            if ancestor != expr_block {
                self.common_subexpressions[*expr_id].on_parent_block = Some(ancestor);
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

        Some(Expression::Variable(
            if common_expression.var_loc.is_some() {
                common_expression.var_loc.unwrap()
            } else {
                Loc::Codegen
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

    /// For common subexpression elimination to work properly, we need to find the common parent of
    /// two blocks. The parent is the deepest block in which every path from the entry block to both
    /// 'block_1' and 'block_2' passes through such a block.
    pub fn find_parent_block(&self, block_1: usize, block_2: usize) -> usize {
        if block_1 == block_2 {
            return block_1;
        }
        let mut colors: Vec<Color> = vec![Color::WHITE; self.cfg_dag.len()];
        let mut visited: Vec<bool> = vec![false; self.cfg_dag.len()];
        /*
        Given a DAG (directed acyclic graph), we color all the ancestors of 'block_1' with yellow.
        Then, we color every ancestor of 'block_2' with blue. As the mixture of blue and yellow
        results in green, green blocks are all possible common ancestors!

        We can't add colors to code. Here, bitwise ORing 2 to a block's color mean painting with yellow.
        Likewise, bitwise ORing 4 means painting with blue. Green blocks have 6 (2|4) as their color
        number.

         */

        self.coloring_dfs(block_1, 0, Color::BLUE, &mut colors, &mut visited);
        visited.fill(false);
        self.coloring_dfs(block_2, 0, Color::YELLOW, &mut colors, &mut visited);

        /*
        Having a bunch of green block, which of them are we looking for?
        We must choose the deepest block, in which all paths from the entry block to both block_1
        and block_2 pass through this block.

        Have a look at the 'find_ancestor' function to know more about the algorithm.
         */
        self.find_ancestor(0, &colors)
    }

    /// Given a colored graph, find the lowest common ancestor.
    fn find_ancestor(&self, start_block: usize, colors: &[Color]) -> usize {
        let mut candidate = start_block;
        let mut queue: VecDeque<usize> = VecDeque::new();
        let mut visited: Vec<bool> = vec![false; self.cfg_dag.len()];

        visited[start_block] = true;
        queue.push_back(start_block);

        let mut six_child: usize = 0;
        // This is a BFS (breadth first search) traversal
        while let Some(cur_block) = queue.pop_front() {
            let mut not_ancestors: usize = 0;
            for child in &self.cfg_dag[cur_block] {
                if colors[*child] == Color::WHITE {
                    // counting the number of children which are not ancestors from neither block_1
                    // nor block_2
                    not_ancestors += 1;
                }

                if colors[*child] == Color::GREEN {
                    // This is the possible candidate to search next.
                    six_child = *child;
                }
            }

            // If the current block has only one child that leads to both block_1 and block_2, it is
            // a candidate to be the lowest common ancestor.
            if not_ancestors + 1 == self.cfg_dag[cur_block].len() && !visited[six_child] {
                visited[six_child] = true;
                queue.push_back(six_child);
                candidate = six_child;
            }
        }

        candidate
    }

    /// This function performs a DFS (depth first search) to color all the ancestors of a block.
    fn coloring_dfs(
        &self,
        search_block: usize,
        cur_block: usize,
        color: Color,
        colors: &mut Vec<Color>,
        visited: &mut Vec<bool>,
    ) -> bool {
        if colors[cur_block].contains(color) {
            return true;
        }

        if visited[cur_block] {
            return false;
        }

        visited[cur_block] = true;
        if cur_block == search_block {
            colors[cur_block].insert(color);
            return true;
        }

        for next in &self.cfg_dag[cur_block] {
            if self.coloring_dfs(search_block, *next, color, colors, visited) {
                colors[cur_block].insert(color);
            }
        }

        colors[cur_block].contains(color)
    }
}
