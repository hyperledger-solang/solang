// SPDX-License-Identifier: Apache-2.0

use crate::sema::{
    ast::*,
    file::PathDisplay,
    symtable::Symtable,
    yul::{
        ast::{YulBlock, YulExpression, YulStatement},
        builtin::YulBuiltInFunction,
    },
};
use solang_parser::{pt, pt::Loc};
use std::fmt::Write;

struct Node {
    name: String,
    labels: Vec<String>,
}

impl Node {
    fn new(name: &str, labels: Vec<String>) -> Self {
        Node {
            name: name.to_owned(),
            labels,
        }
    }
}

struct Edge {
    from: usize,
    to: usize,
    label: Option<String>,
}

struct Dot {
    filename: String,
    nodes: Vec<Node>,
    edges: Vec<Edge>,
}

impl Dot {
    fn write(&self) -> String {
        let mut result = format!("strict digraph \"{}\" {{\n", self.filename);

        for node in &self.nodes {
            if !node.labels.is_empty() {
                writeln!(
                    result,
                    "\t{} [label=\"{}\"]",
                    node.name,
                    node.labels.join("\\n")
                )
                .unwrap();
            }
        }

        for edge in &self.edges {
            if let Some(label) = &edge.label {
                writeln!(
                    result,
                    "\t{} -> {} [label=\"{}\"]",
                    self.nodes[edge.from].name, self.nodes[edge.to].name, label
                )
                .unwrap();
            } else {
                writeln!(
                    result,
                    "\t{} -> {}",
                    self.nodes[edge.from].name, self.nodes[edge.to].name
                )
                .unwrap();
            }
        }

        result.push_str("}\n");

        result
    }

    fn add_node(
        &mut self,
        mut node: Node,
        parent: Option<usize>,
        parent_rel: Option<String>,
    ) -> usize {
        let no = self.nodes.len();

        debug_assert!(
            !node.name.chars().any(|c| c.is_whitespace()),
            "{} contains whitespace",
            node.name
        );

        if node.name.is_empty() || node.name == "node" {
            node.name = format!("node_{no}");
        } else {
            while self.nodes.iter().any(|n| n.name == node.name) {
                node.name = format!("{}_{}", node.name, no);
            }
        }

        self.nodes.push(node);

        if let Some(parent) = parent {
            self.edges.push(Edge {
                from: parent,
                to: no,
                label: parent_rel,
            })
        }

        no
    }

    fn add_tags(&mut self, tags: &[Tag], parent: usize) {
        if !tags.is_empty() {
            let labels = tags
                .iter()
                .map(|tag| format!("{}: {}", tag.tag, tag.value.to_string().escape_debug()))
                .collect();

            self.add_node(
                Node::new("tags", labels),
                Some(parent),
                Some(String::from("tags")),
            );
        }
    }

    fn add_function(&mut self, func: &Function, ns: &Namespace, parent: usize) {
        let mut labels = vec![
            format!("{} {}", func.ty, func.id),
            ns.loc_to_string(PathDisplay::FullPath, &func.loc_prototype),
        ];

        if let Some(contract) = func.contract_no {
            labels.insert(1, format!("contract: {}", ns.contracts[contract].id));
        }

        if func.ty == pt::FunctionTy::Constructor || func.ty == pt::FunctionTy::Function {
            labels.push(format!("signature {}", func.signature));
            labels.push(format!("visibility {}", func.visibility));
        }

        labels.push(format!("mutability {}", func.mutability));

        if func.is_virtual {
            labels.push(String::from("virtual"));
        }

        if let Some((_, is_overrides)) = &func.is_override {
            if is_overrides.is_empty() {
                labels.push(String::from("override"));
            } else {
                for is_override in is_overrides {
                    labels.push(format!("override {}", ns.contracts[*is_override].id));
                }
            }
        }

        if let Some((_, selector)) = &func.selector {
            labels.push(format!("selector {}", hex::encode(selector)));
        }

        let func_node = self.add_node(
            Node::new(&func.id.name, labels),
            Some(parent),
            Some(format!("{}", func.ty)),
        );

        self.add_tags(&func.tags, func_node);

        // parameters
        if !func.params.is_empty() {
            let mut labels = vec![String::from("parameters")];

            for param in &*func.params {
                labels.push(format!(
                    "{}{} {}",
                    if let Some(annotation) = &param.annotation {
                        format!("@{} ", annotation.id.name)
                    } else {
                        String::new()
                    },
                    param.ty.to_string(ns),
                    param.name_as_str()
                ));
            }

            self.add_node(
                Node::new("parameters", labels),
                Some(func_node),
                Some(String::from("parameters")),
            );
        }

        // returns
        if !func.returns.is_empty() {
            let mut labels = vec![String::from("returns")];

            for param in &*func.returns {
                labels.push(format!(
                    "{} {}",
                    param.ty.to_string(ns),
                    param.name_as_str()
                ));
            }

            self.add_node(
                Node::new("returns", labels),
                Some(func_node),
                Some(String::from("returns")),
            );
        }

        // Annotations
        let node = self.add_node(
            Node::new("annotations", vec!["annotations".into()]),
            Some(func_node),
            Some(String::from("annotations")),
        );

        for seed in &func.annotations.seeds {
            self.add_expression(&seed.1, Some(func), ns, node, "seed".into());
        }

        if let Some(space) = &func.annotations.space {
            self.add_expression(&space.1, Some(func), ns, node, "space".into());
        }

        if let Some(bump) = &func.annotations.bump {
            self.add_expression(&bump.1, Some(func), ns, node, "bump".into());
        }

        if let Some((_, name)) = &func.annotations.payer {
            self.add_node(
                Node::new("payer", vec![name.clone()]),
                Some(node),
                Some(String::from("payer declaration")),
            );
        }

        // bases
        for (base_no, (_, _, args)) in &func.bases {
            let node = self.add_node(
                Node::new(
                    &ns.contracts[*base_no].id.name,
                    vec![ns.contracts[*base_no].id.to_string()],
                ),
                Some(func_node),
                Some(String::from("base")),
            );

            for (no, arg) in args.iter().enumerate() {
                self.add_expression(arg, Some(func), ns, node, format!("arg #{no}"));
            }
        }

        // body
        self.add_statement(&func.body, func, ns, func_node, String::from("body"));
    }

    fn add_expression(
        &mut self,
        expr: &Expression,
        func: Option<&Function>,
        ns: &Namespace,
        parent: usize,
        parent_rel: String,
    ) {
        match expr {
            Expression::BoolLiteral { loc, value } => {
                let labels = vec![
                    format!("bool literal: {}", if *value { "true" } else { "false" }),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];

                self.add_node(
                    Node::new("bool_literal", labels),
                    Some(parent),
                    Some(parent_rel),
                );
            }
            Expression::BytesLiteral { loc, ty, value } => {
                let labels = vec![
                    format!("{} literal: {}", ty.to_string(ns), hex::encode(value)),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];

                self.add_node(
                    Node::new("bytes_literal", labels),
                    Some(parent),
                    Some(parent_rel),
                );
            }
            Expression::NumberLiteral { loc, ty, value } => {
                let labels = vec![
                    format!("{} literal: {}", ty.to_string(ns), value),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];

                self.add_node(
                    Node::new("number_literal", labels),
                    Some(parent),
                    Some(parent_rel),
                );
            }
            Expression::RationalNumberLiteral { loc, ty, value } => {
                let labels = vec![
                    format!("rational {} literal: {}", ty.to_string(ns), value),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];

                self.add_node(
                    Node::new("rational_literal", labels),
                    Some(parent),
                    Some(parent_rel),
                );
            }
            Expression::StructLiteral {
                loc, ty, values, ..
            } => {
                let labels = vec![
                    format!("struct literal: {}", ty.to_string(ns)),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];

                let node = self.add_node(
                    Node::new("struct_literal", labels),
                    Some(parent),
                    Some(parent_rel),
                );

                for (no, (_, arg)) in values.iter().enumerate() {
                    self.add_expression(arg, func, ns, node, format!("arg #{no}"));
                }
            }
            Expression::ArrayLiteral {
                loc, ty, values, ..
            } => {
                let labels = vec![
                    format!("array literal: {}", ty.to_string(ns)),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];

                let node = self.add_node(
                    Node::new("array_literal", labels),
                    Some(parent),
                    Some(parent_rel),
                );

                for (no, arg) in values.iter().enumerate() {
                    self.add_expression(arg, func, ns, node, format!("arg #{no}"));
                }
            }
            Expression::ConstArrayLiteral {
                loc, ty, values, ..
            } => {
                let labels = vec![
                    format!("array literal: {}", ty.to_string(ns)),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];

                let node = self.add_node(
                    Node::new("array_literal", labels),
                    Some(parent),
                    Some(parent_rel),
                );

                for (no, arg) in values.iter().enumerate() {
                    self.add_expression(arg, func, ns, node, format!("arg #{no}"));
                }
            }
            Expression::Add {
                loc,
                ty,
                unchecked,
                left,
                right,
            } => {
                let mut labels = vec![
                    String::from("add"),
                    ty.to_string(ns),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];
                if *unchecked {
                    labels.push(String::from("unchecked"));
                }
                let node = self.add_node(Node::new("add", labels), Some(parent), Some(parent_rel));

                self.add_expression(left, func, ns, node, String::from("left"));
                self.add_expression(right, func, ns, node, String::from("right"));
            }
            Expression::Subtract {
                loc,
                ty,
                unchecked,
                left,
                right,
            } => {
                let mut labels = vec![
                    String::from("subtract"),
                    ty.to_string(ns),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];
                if *unchecked {
                    labels.push(String::from("unchecked"));
                }
                let node = self.add_node(
                    Node::new("subtract", labels),
                    Some(parent),
                    Some(parent_rel),
                );

                self.add_expression(left, func, ns, node, String::from("left"));
                self.add_expression(right, func, ns, node, String::from("right"));
            }
            Expression::Multiply {
                loc,
                ty,
                unchecked,
                left,
                right,
            } => {
                let mut labels = vec![
                    String::from("multiply"),
                    ty.to_string(ns),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];
                if *unchecked {
                    labels.push(String::from("unchecked"));
                }
                let node = self.add_node(
                    Node::new("multiply", labels),
                    Some(parent),
                    Some(parent_rel),
                );

                self.add_expression(left, func, ns, node, String::from("left"));
                self.add_expression(right, func, ns, node, String::from("right"));
            }
            Expression::Divide {
                loc,
                ty,
                left,
                right,
            } => {
                let labels = vec![
                    String::from("divide"),
                    ty.to_string(ns),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];
                let node =
                    self.add_node(Node::new("divide", labels), Some(parent), Some(parent_rel));

                self.add_expression(left, func, ns, node, String::from("left"));
                self.add_expression(right, func, ns, node, String::from("right"));
            }
            Expression::Modulo {
                loc,
                ty,
                left,
                right,
            } => {
                let labels = vec![
                    String::from("modulo"),
                    ty.to_string(ns),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];
                let node =
                    self.add_node(Node::new("modulo", labels), Some(parent), Some(parent_rel));

                self.add_expression(left, func, ns, node, String::from("left"));
                self.add_expression(right, func, ns, node, String::from("right"));
            }
            Expression::Power {
                loc,
                ty,
                unchecked,
                base,
                exp,
            } => {
                let mut labels = vec![
                    String::from("power"),
                    ty.to_string(ns),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];
                if *unchecked {
                    labels.push(String::from("unchecked"));
                }
                let node =
                    self.add_node(Node::new("power", labels), Some(parent), Some(parent_rel));

                self.add_expression(base, func, ns, node, String::from("left"));
                self.add_expression(exp, func, ns, node, String::from("right"));
            }
            Expression::BitwiseOr {
                loc,
                ty,
                left,
                right,
            } => {
                let labels = vec![
                    String::from("bitwise or"),
                    ty.to_string(ns),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];
                let node = self.add_node(
                    Node::new("bitwise_or", labels),
                    Some(parent),
                    Some(parent_rel),
                );

                self.add_expression(left, func, ns, node, String::from("left"));
                self.add_expression(right, func, ns, node, String::from("right"));
            }
            Expression::BitwiseAnd {
                loc,
                ty,
                left,
                right,
            } => {
                let labels = vec![
                    String::from("bitwise and"),
                    ty.to_string(ns),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];
                let node = self.add_node(
                    Node::new("bitwise_and", labels),
                    Some(parent),
                    Some(parent_rel),
                );

                self.add_expression(left, func, ns, node, String::from("left"));
                self.add_expression(right, func, ns, node, String::from("right"));
            }
            Expression::BitwiseXor {
                loc,
                ty,
                left,
                right,
            } => {
                let labels = vec![
                    String::from("bitwise xor"),
                    ty.to_string(ns),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];
                let node = self.add_node(
                    Node::new("bitwise_xor", labels),
                    Some(parent),
                    Some(parent_rel),
                );

                self.add_expression(left, func, ns, node, String::from("left"));
                self.add_expression(right, func, ns, node, String::from("right"));
            }
            Expression::ShiftLeft {
                loc,
                ty,
                left,
                right,
            } => {
                let labels = vec![
                    String::from("shift left"),
                    ty.to_string(ns),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];
                let node = self.add_node(
                    Node::new("shift_left", labels),
                    Some(parent),
                    Some(parent_rel),
                );

                self.add_expression(left, func, ns, node, String::from("left"));
                self.add_expression(right, func, ns, node, String::from("right"));
            }
            Expression::ShiftRight {
                loc,
                ty,
                left,
                right,
                ..
            } => {
                let labels = vec![
                    String::from("shift right"),
                    ty.to_string(ns),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];
                let node = self.add_node(
                    Node::new("shift_right", labels),
                    Some(parent),
                    Some(parent_rel),
                );

                self.add_expression(left, func, ns, node, String::from("left"));
                self.add_expression(right, func, ns, node, String::from("right"));
            }
            Expression::ConstantVariable {
                loc,
                ty,
                contract_no,
                var_no,
            } => {
                self.add_constant_variable(loc, ty, contract_no, *var_no, parent, parent_rel, ns);
            }
            Expression::Variable { loc, ty, var_no } => {
                let labels = vec![
                    format!("variable: {}", func.unwrap().symtable.vars[var_no].id.name),
                    ty.to_string(ns),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];

                self.add_node(
                    Node::new("variable", labels),
                    Some(parent),
                    Some(parent_rel),
                );
            }
            Expression::StorageVariable {
                loc,
                ty,
                contract_no,
                var_no,
            } => {
                self.add_storage_variable(loc, ty, *contract_no, *var_no, parent, parent_rel, ns);
            }
            Expression::Load { loc, ty, expr } => {
                let node = self.add_node(
                    Node::new(
                        "load",
                        vec![
                            format!("load {}", ty.to_string(ns)),
                            ns.loc_to_string(PathDisplay::FullPath, loc),
                        ],
                    ),
                    Some(parent),
                    Some(parent_rel),
                );

                self.add_expression(expr, func, ns, node, String::from("expr"));
            }
            Expression::GetRef { loc, ty, expr } => {
                let node = self.add_node(
                    Node::new(
                        "getref",
                        vec![
                            format!("getref {}", ty.to_string(ns)),
                            ns.loc_to_string(PathDisplay::FullPath, loc),
                        ],
                    ),
                    Some(parent),
                    Some(parent_rel),
                );

                self.add_expression(expr, func, ns, node, String::from("expr"));
            }
            Expression::StorageLoad { loc, ty, expr } => {
                let node = self.add_node(
                    Node::new(
                        "storage_load",
                        vec![
                            format!("storage load {}", ty.to_string(ns)),
                            ns.loc_to_string(PathDisplay::FullPath, loc),
                        ],
                    ),
                    Some(parent),
                    Some(parent_rel),
                );

                self.add_expression(expr, func, ns, node, String::from("expr"));
            }
            Expression::ZeroExt { loc, to, expr } => {
                let node = self.add_node(
                    Node::new(
                        "zero_ext",
                        vec![
                            format!("zero extend {}", to.to_string(ns)),
                            ns.loc_to_string(PathDisplay::FullPath, loc),
                        ],
                    ),
                    Some(parent),
                    Some(parent_rel),
                );

                self.add_expression(expr, func, ns, node, String::from("expr"));
            }
            Expression::SignExt { loc, to, expr } => {
                let node = self.add_node(
                    Node::new(
                        "sign_ext",
                        vec![
                            format!("sign extend {}", to.to_string(ns)),
                            ns.loc_to_string(PathDisplay::FullPath, loc),
                        ],
                    ),
                    Some(parent),
                    Some(parent_rel),
                );

                self.add_expression(expr, func, ns, node, String::from("expr"));
            }
            Expression::Trunc { loc, to, expr } => {
                let node = self.add_node(
                    Node::new(
                        "trunc",
                        vec![
                            format!("truncate {}", to.to_string(ns)),
                            ns.loc_to_string(PathDisplay::FullPath, loc),
                        ],
                    ),
                    Some(parent),
                    Some(parent_rel),
                );

                self.add_expression(expr, func, ns, node, String::from("expr"));
            }
            Expression::CheckingTrunc { loc, to, expr } => {
                let node = self.add_node(
                    Node::new(
                        "trunc",
                        vec![
                            format!("checking truncate {}", to.to_string(ns)),
                            ns.loc_to_string(PathDisplay::FullPath, loc),
                        ],
                    ),
                    Some(parent),
                    Some(parent_rel),
                );

                self.add_expression(expr, func, ns, node, String::from("expr"));
            }
            Expression::Cast { loc, to, expr } => {
                let node = self.add_node(
                    Node::new(
                        "cast",
                        vec![
                            format!("cast {}", to.to_string(ns)),
                            ns.loc_to_string(PathDisplay::FullPath, loc),
                        ],
                    ),
                    Some(parent),
                    Some(parent_rel),
                );

                self.add_expression(expr, func, ns, node, String::from("expr"));
            }
            Expression::BytesCast {
                loc,
                to,
                from,
                expr,
            } => {
                let node = self.add_node(
                    Node::new(
                        "bytes_cast",
                        vec![
                            format!(
                                "bytes cast from {} to {}",
                                from.to_string(ns),
                                to.to_string(ns)
                            ),
                            ns.loc_to_string(PathDisplay::FullPath, loc),
                        ],
                    ),
                    Some(parent),
                    Some(parent_rel),
                );

                self.add_expression(expr, func, ns, node, String::from("expr"));
            }
            Expression::PreIncrement {
                loc,
                ty,
                unchecked,
                expr,
            } => {
                let mut labels = vec![
                    String::from("pre increment"),
                    ty.to_string(ns),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];
                if *unchecked {
                    labels.push(String::from("unchecked"));
                }
                let node = self.add_node(
                    Node::new("pre_increment", labels),
                    Some(parent),
                    Some(parent_rel),
                );

                self.add_expression(expr, func, ns, node, String::from("expr"));
            }
            Expression::PreDecrement {
                loc,
                ty,
                unchecked,
                expr,
            } => {
                let mut labels = vec![
                    String::from("pre decrement"),
                    ty.to_string(ns),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];
                if *unchecked {
                    labels.push(String::from("unchecked"));
                }
                let node = self.add_node(
                    Node::new("pre_decrement", labels),
                    Some(parent),
                    Some(parent_rel),
                );

                self.add_expression(expr, func, ns, node, String::from("expr"));
            }
            Expression::PostIncrement {
                loc,
                ty,
                unchecked,
                expr,
            } => {
                let mut labels = vec![
                    String::from("post increment"),
                    ty.to_string(ns),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];
                if *unchecked {
                    labels.push(String::from("unchecked"));
                }
                let node = self.add_node(
                    Node::new("post_increment", labels),
                    Some(parent),
                    Some(parent_rel),
                );

                self.add_expression(expr, func, ns, node, String::from("expr"));
            }
            Expression::PostDecrement {
                loc,
                ty,
                unchecked,
                expr,
            } => {
                let mut labels = vec![
                    String::from("post decrement"),
                    ty.to_string(ns),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];
                if *unchecked {
                    labels.push(String::from("unchecked"));
                }
                let node = self.add_node(
                    Node::new("post_decrement", labels),
                    Some(parent),
                    Some(parent_rel),
                );

                self.add_expression(expr, func, ns, node, String::from("expr"));
            }
            Expression::Assign {
                loc,
                ty,
                left,
                right,
            } => {
                let labels = vec![
                    String::from("assign"),
                    ty.to_string(ns),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];
                let node =
                    self.add_node(Node::new("assign", labels), Some(parent), Some(parent_rel));

                self.add_expression(left, func, ns, node, String::from("left"));
                self.add_expression(right, func, ns, node, String::from("right"));
            }

            Expression::More { loc, left, right } => {
                let labels = vec![
                    String::from("more"),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];
                let node = self.add_node(Node::new("more", labels), Some(parent), Some(parent_rel));

                self.add_expression(left, func, ns, node, String::from("left"));
                self.add_expression(right, func, ns, node, String::from("right"));
            }
            Expression::Less { loc, left, right } => {
                let labels = vec![
                    String::from("less"),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];
                let node = self.add_node(Node::new("less", labels), Some(parent), Some(parent_rel));

                self.add_expression(left, func, ns, node, String::from("left"));
                self.add_expression(right, func, ns, node, String::from("right"));
            }
            Expression::MoreEqual { loc, left, right } => {
                let labels = vec![
                    String::from("more equal"),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];
                let node = self.add_node(
                    Node::new("more_equal", labels),
                    Some(parent),
                    Some(parent_rel),
                );

                self.add_expression(left, func, ns, node, String::from("left"));
                self.add_expression(right, func, ns, node, String::from("right"));
            }
            Expression::LessEqual { loc, left, right } => {
                let labels = vec![
                    String::from("less equal"),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];
                let node = self.add_node(
                    Node::new("less_equal", labels),
                    Some(parent),
                    Some(parent_rel),
                );

                self.add_expression(left, func, ns, node, String::from("left"));
                self.add_expression(right, func, ns, node, String::from("right"));
            }
            Expression::Equal { loc, left, right } => {
                let labels = vec![
                    String::from("equal"),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];
                let node =
                    self.add_node(Node::new("equal", labels), Some(parent), Some(parent_rel));

                self.add_expression(left, func, ns, node, String::from("left"));
                self.add_expression(right, func, ns, node, String::from("right"));
            }
            Expression::NotEqual { loc, left, right } => {
                let labels = vec![
                    String::from("not equal"),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];
                let node = self.add_node(
                    Node::new("not_qual", labels),
                    Some(parent),
                    Some(parent_rel),
                );

                self.add_expression(left, func, ns, node, String::from("left"));
                self.add_expression(right, func, ns, node, String::from("right"));
            }

            Expression::Not { loc, expr } => {
                let node = self.add_node(
                    Node::new(
                        "not",
                        vec![
                            String::from("not"),
                            ns.loc_to_string(PathDisplay::FullPath, loc),
                        ],
                    ),
                    Some(parent),
                    Some(parent_rel),
                );

                self.add_expression(expr, func, ns, node, String::from("expr"));
            }
            Expression::BitwiseNot { loc, ty, expr } => {
                let node = self.add_node(
                    Node::new(
                        "complement",
                        vec![
                            format!("complement {}", ty.to_string(ns)),
                            ns.loc_to_string(PathDisplay::FullPath, loc),
                        ],
                    ),
                    Some(parent),
                    Some(parent_rel),
                );

                self.add_expression(expr, func, ns, node, String::from("expr"));
            }
            Expression::Negate {
                loc,
                ty,
                unchecked,
                expr,
            } => {
                let mut labels = vec![
                    format!("unary minus {}", ty.to_string(ns)),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];
                if *unchecked {
                    labels.push(String::from("unchecked"));
                }

                let node = self.add_node(
                    Node::new("unary_minus", labels),
                    Some(parent),
                    Some(parent_rel),
                );

                self.add_expression(expr, func, ns, node, String::from("expr"));
            }

            Expression::ConditionalOperator {
                loc,
                ty,
                cond,
                true_option: left,
                false_option: right,
            } => {
                let node = self.add_node(
                    Node::new(
                        "conditional",
                        vec![
                            format!("conditional operator {}", ty.to_string(ns)),
                            ns.loc_to_string(PathDisplay::FullPath, loc),
                        ],
                    ),
                    Some(parent),
                    Some(parent_rel),
                );

                self.add_expression(cond, func, ns, node, String::from("cond"));
                self.add_expression(left, func, ns, node, String::from("left"));
                self.add_expression(right, func, ns, node, String::from("right"));
            }
            Expression::Subscript {
                loc,
                array_ty,
                array,
                index,
                ..
            } => {
                let node = self.add_node(
                    Node::new(
                        "subscript",
                        vec![
                            format!("subscript {}", array_ty.to_string(ns)),
                            ns.loc_to_string(PathDisplay::FullPath, loc),
                        ],
                    ),
                    Some(parent),
                    Some(parent_rel),
                );

                self.add_expression(array, func, ns, node, String::from("array"));
                self.add_expression(index, func, ns, node, String::from("index"));
            }
            Expression::StructMember {
                loc,
                ty,
                expr,
                field,
            } => {
                let node = self.add_node(
                    Node::new(
                        "structmember",
                        vec![
                            format!("struct member #{} {}", field, ty.to_string(ns)),
                            ns.loc_to_string(PathDisplay::FullPath, loc),
                        ],
                    ),
                    Some(parent),
                    Some(parent_rel),
                );

                self.add_expression(expr, func, ns, node, String::from("var"));
            }

            Expression::AllocDynamicBytes {
                loc,
                ty,
                length,
                init,
            } => {
                let mut labels = vec![
                    format!("alloc array {}", ty.to_string(ns)),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];

                if let Some(initializer) = init {
                    labels.insert(1, format!("initializer: {}", hex::encode(initializer)));
                }

                let node = self.add_node(
                    Node::new("alloc_array", labels),
                    Some(parent),
                    Some(parent_rel),
                );

                self.add_expression(length, func, ns, node, String::from("length"));
            }

            Expression::StorageArrayLength {
                loc,
                ty,
                array,
                elem_ty,
            } => {
                let node = self.add_node(
                    Node::new(
                        "array_length",
                        vec![
                            format!("array length {}", ty.to_string(ns)),
                            format!("element {}", elem_ty.to_string(ns)),
                            ns.loc_to_string(PathDisplay::FullPath, loc),
                        ],
                    ),
                    Some(parent),
                    Some(parent_rel),
                );

                self.add_expression(array, func, ns, node, String::from("array"));
            }
            Expression::StringCompare { loc, left, right } => {
                let node = self.add_node(
                    Node::new(
                        "string_cmp",
                        vec![
                            String::from("string compare"),
                            ns.loc_to_string(PathDisplay::FullPath, loc),
                        ],
                    ),
                    Some(parent),
                    Some(parent_rel),
                );

                self.add_string_location(left, func, ns, node, String::from("left"));
                self.add_string_location(right, func, ns, node, String::from("right"));
            }
            Expression::Or { loc, left, right } => {
                let labels = vec![
                    String::from("logical or"),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];
                let node = self.add_node(
                    Node::new("logical_or", labels),
                    Some(parent),
                    Some(parent_rel),
                );

                self.add_expression(left, func, ns, node, String::from("left"));
                self.add_expression(right, func, ns, node, String::from("right"));
            }
            Expression::And { loc, left, right } => {
                let labels = vec![
                    String::from("logical and"),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];
                let node = self.add_node(
                    Node::new("logical_and", labels),
                    Some(parent),
                    Some(parent_rel),
                );

                self.add_expression(left, func, ns, node, String::from("left"));
                self.add_expression(right, func, ns, node, String::from("right"));
            }

            Expression::InternalFunction {
                loc,
                ty,
                function_no,
                signature,
                ..
            } => {
                let mut labels = vec![
                    ty.to_string(ns),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];

                let func = &ns.functions[*function_no];

                if let Some(contract_no) = func.contract_no {
                    labels.insert(1, format!("{}.{}", ns.contracts[contract_no].id, func.id))
                } else {
                    labels.insert(1, format!("free function {}", func.id))
                }

                if let Some(signature) = signature {
                    labels.insert(1, format!("signature {signature}"))
                }

                self.add_node(
                    Node::new("internal_function", labels),
                    Some(parent),
                    Some(parent_rel),
                );
            }
            Expression::ExternalFunction {
                loc,
                ty,
                function_no,
                address,
            } => {
                let mut labels = vec![
                    ty.to_string(ns),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];

                let f = &ns.functions[*function_no];

                if let Some(contract_no) = f.contract_no {
                    labels.insert(1, format!("{}.{}", ns.contracts[contract_no].id, f.id))
                }

                let node = self.add_node(
                    Node::new("external_function", labels),
                    Some(parent),
                    Some(parent_rel),
                );

                self.add_expression(address, func, ns, node, String::from("address"));
            }
            Expression::InternalFunctionCall {
                loc,
                function,
                args,
                ..
            } => {
                let labels = vec![
                    String::from("call internal function"),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];

                let node = self.add_node(
                    Node::new("call_internal_function", labels),
                    Some(parent),
                    Some(parent_rel),
                );

                self.add_expression(function, func, ns, node, String::from("function"));

                for (no, arg) in args.iter().enumerate() {
                    self.add_expression(arg, func, ns, node, format!("arg #{no}"));
                }
            }
            Expression::ExternalFunctionCall {
                loc,
                function,
                args,
                call_args,
                ..
            } => {
                let labels = vec![
                    String::from("call external function"),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];

                let node = self.add_node(
                    Node::new("call_external_function", labels),
                    Some(parent),
                    Some(parent_rel),
                );

                self.add_expression(function, func, ns, node, String::from("function"));

                for (no, arg) in args.iter().enumerate() {
                    self.add_expression(arg, func, ns, node, format!("arg #{no}"));
                }

                self.add_call_args(call_args, func, ns, node);
            }
            Expression::ExternalFunctionCallRaw {
                loc,
                address,
                args,
                call_args,
                ..
            } => {
                let labels = vec![
                    String::from("call external function"),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];

                let node = self.add_node(
                    Node::new("call_external_function", labels),
                    Some(parent),
                    Some(parent_rel),
                );

                self.add_expression(address, func, ns, node, String::from("address"));
                self.add_expression(args, func, ns, node, String::from("args"));
                self.add_call_args(call_args, func, ns, node);
            }
            Expression::Constructor {
                loc,
                contract_no,
                args,
                call_args,
                ..
            } => {
                let labels = vec![
                    format!("constructor contract {}", ns.contracts[*contract_no].id),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];

                let node = self.add_node(
                    Node::new("constructor", labels),
                    Some(parent),
                    Some(parent_rel),
                );

                for (no, arg) in args.iter().enumerate() {
                    self.add_expression(arg, func, ns, node, format!("arg #{no}"));
                }

                self.add_call_args(call_args, func, ns, node);
            }

            Expression::FormatString { loc, format } => {
                let labels = vec![
                    String::from("string format"),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];

                let node = self.add_node(
                    Node::new("string_format", labels),
                    Some(parent),
                    Some(parent_rel),
                );

                for (no, (_, arg)) in format.iter().enumerate() {
                    self.add_expression(arg, func, ns, node, format!("arg #{no}"));
                }
            }
            Expression::Builtin {
                loc, kind, args, ..
            } => {
                let labels = vec![
                    format!("builtin {kind:?}"),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];

                let node = self.add_node(
                    Node::new("builtins", labels),
                    Some(parent),
                    Some(parent_rel),
                );

                for (no, arg) in args.iter().enumerate() {
                    self.add_expression(arg, func, ns, node, format!("arg #{no}"));
                }
            }
            Expression::UserDefinedOperator {
                loc,
                oper,
                function_no,
                args,
                ..
            } => {
                let user_func = &ns.functions[*function_no];

                let labels = vec![
                    format!(
                        "user defined operator {} {}",
                        oper,
                        ns.loc_to_string(PathDisplay::FullPath, loc)
                    ),
                    format!(
                        "function {} {}",
                        user_func.id,
                        ns.loc_to_string(PathDisplay::FullPath, &user_func.loc_prototype)
                    ),
                ];

                let node = self.add_node(
                    Node::new("user_defined_operator", labels),
                    Some(parent),
                    Some(parent_rel),
                );

                for (no, arg) in args.iter().enumerate() {
                    self.add_expression(arg, func, ns, node, format!("arg #{no}"));
                }
            }
            Expression::List { loc, list } => {
                let labels = vec![
                    String::from("list"),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];

                let node = self.add_node(Node::new("list", labels), Some(parent), Some(parent_rel));

                for (no, expr) in list.iter().enumerate() {
                    self.add_expression(expr, func, ns, node, format!("entry #{no}"));
                }
            }
            Expression::NamedMember {
                loc, name, array, ..
            } => {
                let labels = vec![
                    format!("named array access: {}", name),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];

                let node = self.add_node(
                    Node::new("named_subscript", labels),
                    Some(parent),
                    Some(parent_rel),
                );
                self.add_expression(array, func, ns, node, format!("member: {}", name));
            }
            Expression::EventSelector { loc, event_no, .. } => {
                let event = &ns.events[*event_no];

                let labels = vec![
                    format!("event selector {}", event.symbol_name(ns)),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];

                self.add_node(
                    Node::new("event_selector", labels),
                    Some(parent),
                    Some(parent_rel),
                );
            }
            Expression::TypeOperator { loc, ty } => {
                let labels = vec![
                    format!("type({})", ty.to_string(ns)),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];

                self.add_node(
                    Node::new("type_operator", labels),
                    Some(parent),
                    Some(parent_rel),
                );
            }
        }
    }

    fn add_call_args(
        &mut self,
        call_args: &CallArgs,
        func: Option<&Function>,
        ns: &Namespace,
        node: usize,
    ) {
        if let Some(gas) = &call_args.gas {
            self.add_expression(gas, func, ns, node, String::from("gas"));
        }
        if let Some(value) = &call_args.value {
            self.add_expression(value, func, ns, node, String::from("value"));
        }
        if let Some(salt) = &call_args.salt {
            self.add_expression(salt, func, ns, node, String::from("salt"));
        }
        if let ExternalCallAccounts::Present(accounts) = &call_args.accounts {
            self.add_expression(accounts, func, ns, node, String::from("accounts"));
        }
        if let Some(seeds) = &call_args.seeds {
            self.add_expression(seeds, func, ns, node, String::from("seeds"));
        }
        if let Some(flags) = &call_args.flags {
            self.add_expression(flags, func, ns, node, String::from("flags"));
        }
    }

    fn add_string_location(
        &mut self,
        loc: &StringLocation<Expression>,
        func: Option<&Function>,
        ns: &Namespace,
        parent: usize,
        parent_rel: String,
    ) {
        match loc {
            StringLocation::CompileTime(val) => {
                self.add_node(
                    Node::new(
                        "compile_time_string",
                        vec![format!("const string {}", hex::encode(val))],
                    ),
                    Some(parent),
                    Some(parent_rel),
                );
            }
            StringLocation::RunTime(expr) => {
                self.add_expression(expr, func, ns, parent, parent_rel);
            }
        }
    }

    fn add_statement(
        &mut self,
        stmts: &[Statement],
        func: &Function,
        ns: &Namespace,
        parent: usize,
        parent_rel: String,
    ) {
        let mut parent = parent;
        let mut parent_rel = parent_rel;

        for stmt in stmts {
            match stmt {
                Statement::Block {
                    loc,
                    unchecked,
                    statements,
                } => {
                    let mut labels = vec![
                        String::from("block"),
                        ns.loc_to_string(PathDisplay::FullPath, loc),
                    ];

                    if *unchecked {
                        labels.push(String::from("unchecked"));
                    }

                    parent =
                        self.add_node(Node::new("block", labels), Some(parent), Some(parent_rel));

                    self.add_statement(statements, func, ns, parent, String::from("statements"));
                }
                Statement::VariableDecl(loc, _, param, init) => {
                    let labels = vec![
                        format!(
                            "variable decl {} {}",
                            param.ty.to_string(ns),
                            param.name_as_str()
                        ),
                        ns.loc_to_string(PathDisplay::FullPath, loc),
                    ];

                    parent = self.add_node(
                        Node::new("var_decl", labels),
                        Some(parent),
                        Some(parent_rel),
                    );

                    if let Some(init) = init {
                        self.add_expression(init, Some(func), ns, parent, String::from("init"));
                    }
                }
                Statement::If(loc, _, cond, then, else_) => {
                    let labels = vec![
                        String::from("if"),
                        ns.loc_to_string(PathDisplay::FullPath, loc),
                    ];

                    parent = self.add_node(Node::new("if", labels), Some(parent), Some(parent_rel));

                    self.add_expression(cond, Some(func), ns, parent, String::from("cond"));
                    self.add_statement(then, func, ns, parent, String::from("then"));
                    self.add_statement(else_, func, ns, parent, String::from("else"));
                }
                Statement::While(loc, _, cond, body) => {
                    let labels = vec![
                        String::from("while"),
                        ns.loc_to_string(PathDisplay::FullPath, loc),
                    ];

                    parent =
                        self.add_node(Node::new("while", labels), Some(parent), Some(parent_rel));

                    self.add_expression(cond, Some(func), ns, parent, String::from("cond"));
                    self.add_statement(body, func, ns, parent, String::from("body"));
                }
                Statement::For {
                    loc,
                    init,
                    cond,
                    next,
                    body,
                    ..
                } => {
                    let labels = vec![
                        String::from("for"),
                        ns.loc_to_string(PathDisplay::FullPath, loc),
                    ];

                    parent =
                        self.add_node(Node::new("for", labels), Some(parent), Some(parent_rel));

                    self.add_statement(init, func, ns, parent, String::from("init"));
                    if let Some(cond) = cond {
                        self.add_expression(cond, Some(func), ns, parent, String::from("cond"));
                    }
                    if let Some(next) = next {
                        self.add_expression(next, Some(func), ns, parent, String::from("next"));
                    }
                    self.add_statement(body, func, ns, parent, String::from("body"));
                }
                Statement::DoWhile(loc, _, body, cond) => {
                    let labels = vec![
                        String::from("do while"),
                        ns.loc_to_string(PathDisplay::FullPath, loc),
                    ];

                    parent =
                        self.add_node(Node::new("dowhile", labels), Some(parent), Some(parent_rel));

                    self.add_statement(body, func, ns, parent, String::from("body"));
                    self.add_expression(cond, Some(func), ns, parent, String::from("cond"));
                }
                Statement::Expression(loc, _, expr) => {
                    let labels = vec![
                        String::from("expression"),
                        ns.loc_to_string(PathDisplay::FullPath, loc),
                    ];

                    parent =
                        self.add_node(Node::new("expr", labels), Some(parent), Some(parent_rel));

                    self.add_expression(expr, Some(func), ns, parent, String::from("expr"));
                }
                Statement::Delete(loc, ty, expr) => {
                    let labels = vec![
                        String::from("delete"),
                        format!("ty: {}", ty.to_string(ns)),
                        ns.loc_to_string(PathDisplay::FullPath, loc),
                    ];

                    parent =
                        self.add_node(Node::new("delete", labels), Some(parent), Some(parent_rel));

                    self.add_expression(expr, Some(func), ns, parent, String::from("expr"));
                }
                Statement::Destructure(loc, fields, expr) => {
                    let labels = vec![
                        String::from("destructure"),
                        ns.loc_to_string(PathDisplay::FullPath, loc),
                    ];

                    parent = self.add_node(
                        Node::new("destructure", labels),
                        Some(parent),
                        Some(parent_rel),
                    );

                    for (no, field) in fields.iter().enumerate() {
                        let parent_rel = format!("arg #{no}");

                        match field {
                            DestructureField::None => {
                                self.add_node(
                                    Node::new("none", vec![String::from("none")]),
                                    Some(parent),
                                    Some(parent_rel),
                                );
                            }
                            DestructureField::Expression(expr) => {
                                self.add_expression(expr, Some(func), ns, parent, parent_rel);
                            }
                            DestructureField::VariableDecl(_, param) => {
                                self.add_node(
                                    Node::new(
                                        "param",
                                        vec![format!(
                                            "{} {}",
                                            param.ty.to_string(ns),
                                            param.name_as_str()
                                        )],
                                    ),
                                    Some(parent),
                                    Some(parent_rel),
                                );
                            }
                        }
                    }

                    self.add_expression(expr, Some(func), ns, parent, String::from("expr"));
                }
                Statement::Continue(loc) => {
                    let labels = vec![
                        String::from("continue"),
                        ns.loc_to_string(PathDisplay::FullPath, loc),
                    ];

                    parent = self.add_node(
                        Node::new("continue", labels),
                        Some(parent),
                        Some(parent_rel),
                    );
                }
                Statement::Break(loc) => {
                    let labels = vec![
                        String::from("break"),
                        ns.loc_to_string(PathDisplay::FullPath, loc),
                    ];

                    parent =
                        self.add_node(Node::new("break", labels), Some(parent), Some(parent_rel));
                }
                Statement::Return(loc, expr) => {
                    let labels = vec![
                        String::from("return"),
                        ns.loc_to_string(PathDisplay::FullPath, loc),
                    ];

                    parent =
                        self.add_node(Node::new("return", labels), Some(parent), Some(parent_rel));

                    if let Some(expr) = expr {
                        self.add_expression(expr, Some(func), ns, parent, String::from("expr"));
                    }
                }
                Statement::Emit {
                    loc,
                    event_no,
                    args,
                    ..
                } => {
                    let mut labels = vec![
                        String::from("emit"),
                        ns.loc_to_string(PathDisplay::FullPath, loc),
                    ];

                    let event = &ns.events[*event_no];

                    labels.insert(1, format!("event {}", event.symbol_name(ns)));

                    parent =
                        self.add_node(Node::new("emit", labels), Some(parent), Some(parent_rel));

                    for (no, arg) in args.iter().enumerate() {
                        self.add_expression(arg, Some(func), ns, parent, format!("arg #{no}"));
                    }
                }
                Statement::Revert {
                    loc,
                    error_no,
                    args,
                    ..
                } => {
                    let mut labels = vec![
                        String::from("revert"),
                        ns.loc_to_string(PathDisplay::FullPath, loc),
                    ];

                    if let Some(error_no) = error_no {
                        let error = &ns.errors[*error_no];

                        labels.insert(1, format!("error {}", error.symbol_name(ns)));
                    }

                    parent =
                        self.add_node(Node::new("revert", labels), Some(parent), Some(parent_rel));

                    for (no, arg) in args.iter().enumerate() {
                        self.add_expression(arg, Some(func), ns, parent, format!("arg #{no}"));
                    }
                }
                Statement::TryCatch(loc, _, try_catch) => {
                    let labels = vec![
                        String::from("try"),
                        ns.loc_to_string(PathDisplay::FullPath, loc),
                    ];

                    self.add_expression(
                        &try_catch.expr,
                        Some(func),
                        ns,
                        parent,
                        String::from("expr"),
                    );

                    parent =
                        self.add_node(Node::new("try", labels), Some(parent), Some(parent_rel));

                    for (no, (_, param)) in try_catch.returns.iter().enumerate() {
                        let parent_rel = format!("return #{no}");

                        self.add_node(
                            Node::new(
                                "return",
                                vec![format!(
                                    "{} {}",
                                    param.ty.to_string(ns),
                                    param.name_as_str()
                                )],
                            ),
                            Some(parent),
                            Some(parent_rel),
                        );
                    }

                    self.add_statement(&try_catch.ok_stmt, func, ns, parent, String::from("ok"));

                    for clause in &try_catch.errors {
                        self.add_node(
                            Node::new(
                                "error_param",
                                vec![format!(
                                    "{} {}",
                                    clause.param.as_ref().unwrap().ty.to_string(ns),
                                    clause.param.as_ref().unwrap().name_as_str()
                                )],
                            ),
                            Some(parent),
                            Some(String::from("error parameter")),
                        );

                        self.add_statement(&clause.stmt, func, ns, parent, String::from("error"));
                    }

                    if let Some(clause) = try_catch
                        .catch_all
                        .as_ref()
                        .filter(|clause| clause.param.is_some())
                    {
                        let param = clause.param.as_ref().unwrap();
                        let label = format!("{} {}", param.ty.to_string(ns), param.name_as_str());
                        let node = Node::new("catch_param", vec![label]);
                        self.add_node(node, Some(parent), Some(String::from("catch parameter")));
                    }

                    if let Some(clause) = try_catch.catch_all.as_ref() {
                        self.add_statement(&clause.stmt, func, ns, parent, String::from("catch"));
                    }
                }
                Statement::Underscore(loc) => {
                    let labels = vec![
                        String::from("undersore"),
                        ns.loc_to_string(PathDisplay::FullPath, loc),
                    ];

                    parent = self.add_node(
                        Node::new("underscore", labels),
                        Some(parent),
                        Some(parent_rel),
                    );
                }
                Statement::Assembly(inline_assembly, ..) => {
                    let labels = vec![
                        "inline assembly".to_string(),
                        ns.loc_to_string(PathDisplay::FullPath, &inline_assembly.loc),
                    ];
                    parent = self.add_node(
                        Node::new("inline_assembly", labels),
                        Some(parent),
                        Some(parent_rel),
                    );

                    let mut local_parent = parent;

                    for n in inline_assembly.functions.start..inline_assembly.functions.end {
                        self.add_yul_function(
                            n,
                            ns,
                            local_parent,
                            format!("func def #{}", inline_assembly.functions.end - n),
                        );
                    }

                    local_parent = parent;
                    for (item_no, item) in inline_assembly.body.iter().enumerate() {
                        local_parent = self.add_yul_statement(
                            item,
                            local_parent,
                            format!("statement #{item_no}"),
                            &func.symtable,
                            ns,
                        );
                    }
                }
            }
            parent_rel = String::from("next");
        }
    }

    fn add_yul_function(
        &mut self,
        func_no: usize,
        ns: &Namespace,
        parent: usize,
        parent_rel: String,
    ) {
        let labels = vec![
            format!("function definition {}", ns.yul_functions[func_no].name),
            ns.loc_to_string(PathDisplay::FullPath, &ns.yul_functions[func_no].loc),
        ];

        let func_node = self.add_node(
            Node::new("yul_function_definition", labels),
            Some(parent),
            Some(parent_rel),
        );

        let mut local_parent = func_node;
        for (item_no, item) in (*ns.yul_functions[func_no].params).iter().enumerate() {
            let labels = vec![
                format!(
                    "function parameter {}: {}",
                    item.ty.to_string(ns),
                    item.id.as_ref().unwrap().name,
                ),
                ns.loc_to_string(PathDisplay::FullPath, &item.loc),
            ];
            local_parent = self.add_node(
                Node::new("yul_function_parameter", labels),
                Some(local_parent),
                Some(format!("parameter #{item_no}")),
            );
        }

        local_parent = func_node;
        for (item_no, item) in (*ns.yul_functions[func_no].returns).iter().enumerate() {
            let labels = vec![
                format!(
                    "return parameter {}: {}",
                    item.ty.to_string(ns),
                    item.id.as_ref().unwrap().name
                ),
                ns.loc_to_string(PathDisplay::FullPath, &item.loc),
            ];
            local_parent = self.add_node(
                Node::new("yul_function_return", labels),
                Some(local_parent),
                Some(format!("return #{item_no}")),
            );
        }

        local_parent = func_node;
        for (item_no, item) in ns.yul_functions[func_no].body.statements.iter().enumerate() {
            local_parent = self.add_yul_statement(
                item,
                local_parent,
                format!("statement #{item_no}"),
                &ns.yul_functions[func_no].symtable,
                ns,
            );
        }
    }

    fn add_yul_expression(
        &mut self,
        expr: &YulExpression,
        symtable: &Symtable,
        ns: &Namespace,
        parent: usize,
        parent_rel: String,
    ) {
        match expr {
            YulExpression::BoolLiteral(loc, value, ty) => {
                let labels = vec![
                    format!(
                        "bool literal: {} of type {}",
                        if *value { "true" } else { "false" },
                        ty.to_string(ns)
                    ),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];

                self.add_node(
                    Node::new("yul_bool_literal", labels),
                    Some(parent),
                    Some(parent_rel),
                );
            }
            YulExpression::NumberLiteral(loc, value, ty) => {
                let labels = vec![
                    format!("{} literal: {}", ty.to_string(ns), value),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];

                self.add_node(
                    Node::new("yul_number_literal", labels),
                    Some(parent),
                    Some(parent_rel),
                );
            }
            YulExpression::StringLiteral(loc, value, ty) => {
                let labels = vec![
                    format!("{} literal: {}", ty.to_string(ns), hex::encode(value)),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];

                self.add_node(
                    Node::new("bytes_literal", labels),
                    Some(parent),
                    Some(parent_rel),
                );
            }
            YulExpression::YulLocalVariable(loc, ty, var_no) => {
                let labels = vec![
                    format!("yul variable: {}", symtable.vars[var_no].id.name),
                    ty.to_string(ns),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];
                self.add_node(
                    Node::new("yul_variable", labels),
                    Some(parent),
                    Some(parent_rel),
                );
            }
            YulExpression::SolidityLocalVariable(loc, ty, _, var_no) => {
                let labels = vec![
                    format!("solidity variable: {}", symtable.vars[var_no].id.name),
                    ty.to_string(ns),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];

                self.add_node(
                    Node::new("solidity_variable", labels),
                    Some(parent),
                    Some(parent_rel),
                );
            }
            YulExpression::ConstantVariable(loc, ty, contract, var_no) => {
                self.add_constant_variable(loc, ty, contract, *var_no, parent, parent_rel, ns);
            }
            YulExpression::StorageVariable(loc, ty, contract, var_no) => {
                self.add_storage_variable(loc, ty, *contract, *var_no, parent, parent_rel, ns);
            }
            YulExpression::BuiltInCall(loc, builtin_ty, args) => {
                self.add_yul_builtin_call(loc, *builtin_ty, args, parent, parent_rel, symtable, ns);
            }
            YulExpression::FunctionCall(loc, func_no, args, _) => {
                self.add_yul_function_call(loc, *func_no, args, parent, parent_rel, symtable, ns);
            }
            YulExpression::SuffixAccess(loc, member, suffix) => {
                let labels = vec![
                    format!("yul suffix '{}' access", suffix.to_string()),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];

                let node = self.add_node(
                    Node::new("yul_suffix_access", labels),
                    Some(parent),
                    Some(parent_rel),
                );

                self.add_yul_expression(member, symtable, ns, node, "parent".to_string());
            }
        }
    }

    fn add_constant_variable(
        &mut self,
        loc: &Loc,
        ty: &Type,
        contract: &Option<usize>,
        var_no: usize,
        parent: usize,
        parent_rel: String,
        ns: &Namespace,
    ) {
        let mut labels = vec![
            String::from("constant variable"),
            ty.to_string(ns),
            ns.loc_to_string(PathDisplay::FullPath, loc),
        ];

        if let Some(contract) = contract {
            labels.insert(
                1,
                format!(
                    "{}.{}",
                    ns.contracts[*contract].id, ns.contracts[*contract].variables[var_no].name
                ),
            );
        } else {
            labels.insert(1, ns.constants[var_no].name.to_string());
        }

        self.add_node(
            Node::new("constant", labels),
            Some(parent),
            Some(parent_rel),
        );
    }

    fn add_storage_variable(
        &mut self,
        loc: &Loc,
        ty: &Type,
        contract: usize,
        var_no: usize,
        parent: usize,
        parent_rel: String,
        ns: &Namespace,
    ) {
        let labels = vec![
            String::from("storage variable"),
            format!(
                "{}.{}",
                ns.contracts[contract].id, ns.contracts[contract].variables[var_no].name
            ),
            ty.to_string(ns),
            ns.loc_to_string(PathDisplay::FullPath, loc),
        ];

        self.add_node(
            Node::new("storage_var", labels),
            Some(parent),
            Some(parent_rel),
        );
    }

    fn add_yul_statement(
        &mut self,
        statement: &YulStatement,
        parent: usize,
        parent_rel: String,
        symtable: &Symtable,
        ns: &Namespace,
    ) -> usize {
        match statement {
            YulStatement::FunctionCall(loc, _, func_no, args) => {
                self.add_yul_function_call(loc, *func_no, args, parent, parent_rel, symtable, ns)
            }
            YulStatement::BuiltInCall(loc, _, builtin_ty, args) => {
                self.add_yul_builtin_call(loc, *builtin_ty, args, parent, parent_rel, symtable, ns)
            }
            YulStatement::Block(block) => {
                self.add_yul_block(block, parent, parent_rel, symtable, ns)
            }
            YulStatement::VariableDeclaration(loc, _, declared_vars, initializer) => {
                let labels = vec![
                    "yul variable declaration".to_string(),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];

                let node = self.add_node(
                    Node::new("yul_var_decl", labels),
                    Some(parent),
                    Some(parent_rel),
                );

                for (decl_no, item) in declared_vars.iter().enumerate() {
                    let var = &symtable.vars[&item.0];
                    self.add_node(
                        Node::new(
                            "var_decl_item",
                            vec![
                                format!(
                                    "yul variable declaration {} {}",
                                    var.ty.to_string(ns),
                                    var.id.name
                                ),
                                ns.loc_to_string(PathDisplay::FullPath, &var.id.loc),
                            ],
                        ),
                        Some(node),
                        Some(format!("decl item #{decl_no}")),
                    );
                }

                if let Some(init) = initializer {
                    self.add_yul_expression(init, symtable, ns, node, "init".to_string());
                }

                node
            }
            YulStatement::Assignment(loc, _, lhs, rhs) => {
                let labels = vec![
                    "yul assignment".to_string(),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];

                let node = self.add_node(
                    Node::new("yul_assignment", labels),
                    Some(parent),
                    Some(parent_rel),
                );

                for (item_no, item) in lhs.iter().enumerate() {
                    self.add_yul_expression(item, symtable, ns, node, format!("rhs #{item_no}"));
                }

                self.add_yul_expression(rhs, symtable, ns, node, "lhs".to_string());
                node
            }
            YulStatement::IfBlock(loc, _, condition, block) => {
                let labels = vec![
                    "yul if".to_string(),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];

                let node = self.add_node(Node::new("if", labels), Some(parent), Some(parent_rel));

                self.add_yul_expression(condition, symtable, ns, node, "cond".to_string());
                self.add_yul_block(block, node, "if-block".to_string(), symtable, ns);
                node
            }
            YulStatement::Switch {
                loc,
                condition,
                cases,
                default,
                ..
            } => {
                let labels = vec![
                    "yul switch".to_string(),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];

                let node =
                    self.add_node(Node::new("switch", labels), Some(parent), Some(parent_rel));

                self.add_yul_expression(condition, symtable, ns, node, "cond".to_string());

                for (item_no, item) in cases.iter().enumerate() {
                    let case_block = self.add_node(
                        Node::new(
                            "case",
                            vec![
                                "yul switch case".to_string(),
                                ns.loc_to_string(PathDisplay::FullPath, &item.loc),
                            ],
                        ),
                        Some(node),
                        Some(format!("case #{item_no}")),
                    );
                    self.add_yul_expression(
                        &item.condition,
                        symtable,
                        ns,
                        case_block,
                        "case-condition".to_string(),
                    );
                    self.add_yul_block(
                        &item.block,
                        case_block,
                        "case block".to_string(),
                        symtable,
                        ns,
                    );
                }

                if let Some(default_block) = default {
                    let default_node = self.add_node(
                        Node::new(
                            "default",
                            vec![
                                "yul switch default".to_string(),
                                ns.loc_to_string(PathDisplay::FullPath, &default_block.loc),
                            ],
                        ),
                        Some(node),
                        Some("default".to_string()),
                    );
                    self.add_yul_block(
                        default_block,
                        default_node,
                        "default block".to_string(),
                        symtable,
                        ns,
                    );
                }
                node
            }
            YulStatement::For {
                loc,
                init_block,
                condition,
                post_block,
                execution_block,
                ..
            } => {
                let labels = vec![
                    "yul for".to_string(),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];

                let node = self.add_node(Node::new("for", labels), Some(parent), Some(parent_rel));

                self.add_yul_block(init_block, node, "init block".to_string(), symtable, ns);
                self.add_yul_expression(condition, symtable, ns, node, "for condition".to_string());
                self.add_yul_block(post_block, node, "post block".to_string(), symtable, ns);
                self.add_yul_block(
                    execution_block,
                    node,
                    "execution block".to_string(),
                    symtable,
                    ns,
                );
                node
            }
            YulStatement::Leave(loc, _) => {
                let labels = vec![
                    "leave".to_string(),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];
                self.add_node(Node::new("leave", labels), Some(parent), Some(parent_rel))
            }
            YulStatement::Break(loc, _) => {
                let labels = vec![
                    "break".to_string(),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];
                self.add_node(Node::new("break", labels), Some(parent), Some(parent_rel))
            }
            YulStatement::Continue(loc, _) => {
                let labels = vec![
                    "continue".to_string(),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];
                self.add_node(
                    Node::new("continue", labels),
                    Some(parent),
                    Some(parent_rel),
                )
            }
        }
    }

    fn add_yul_block(
        &mut self,
        block: &YulBlock,
        mut parent: usize,
        parent_rel: String,
        symtable: &Symtable,
        ns: &Namespace,
    ) -> usize {
        let label = vec![
            "assembly block".to_string(),
            ns.loc_to_string(PathDisplay::FullPath, &block.loc),
        ];

        let node = self.add_node(
            Node::new("assembly_block", label),
            Some(parent),
            Some(parent_rel),
        );

        parent = node;
        for (statement_no, child_statement) in block.statements.iter().enumerate() {
            parent = self.add_yul_statement(
                child_statement,
                parent,
                format!("statement #{statement_no}"),
                symtable,
                ns,
            );
        }

        node
    }

    fn add_yul_function_call(
        &mut self,
        loc: &Loc,
        func_no: usize,
        args: &[YulExpression],
        parent: usize,
        parent_rel: String,
        symtable: &Symtable,
        ns: &Namespace,
    ) -> usize {
        let labels = vec![
            format!("yul function call '{}'", ns.yul_functions[func_no].name),
            ns.loc_to_string(PathDisplay::FullPath, loc),
        ];

        let node = self.add_node(
            Node::new("yul_function_call", labels),
            Some(parent),
            Some(parent_rel),
        );

        for (arg_no, arg) in args.iter().enumerate() {
            self.add_yul_expression(arg, symtable, ns, node, format!("arg #{arg_no}"));
        }

        node
    }

    fn add_yul_builtin_call(
        &mut self,
        loc: &Loc,
        builtin_ty: YulBuiltInFunction,
        args: &[YulExpression],
        parent: usize,
        parent_rel: String,
        symtable: &Symtable,
        ns: &Namespace,
    ) -> usize {
        let labels = vec![
            format!("yul builtin call '{}'", builtin_ty.to_string()),
            ns.loc_to_string(PathDisplay::FullPath, loc),
        ];

        let node = self.add_node(
            Node::new("yul_builtin_call", labels),
            Some(parent),
            Some(parent_rel),
        );

        for (arg_no, arg) in args.iter().enumerate() {
            self.add_yul_expression(arg, symtable, ns, node, format!("arg #{arg_no}"));
        }

        node
    }

    fn add_version(
        &mut self,
        version: &VersionReq,
        parent: usize,
        parent_rel: String,
        ns: &Namespace,
    ) {
        match version {
            VersionReq::Plain { loc, version } => {
                let labels = vec![
                    format!("version: {version}"),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];

                self.add_node(Node::new("plain", labels), Some(parent), Some(parent_rel));
            }
            VersionReq::Operator { loc, op, version } => {
                let labels = vec![
                    format!("version: {op}{version}"),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];

                self.add_node(
                    Node::new("operator", labels),
                    Some(parent),
                    Some(parent_rel),
                );
            }
            VersionReq::Range { loc, from, to } => {
                let labels = vec![
                    format!("version: {from} - {to}"),
                    ns.loc_to_string(PathDisplay::FullPath, loc),
                ];

                self.add_node(Node::new("range", labels), Some(parent), Some(parent_rel));
            }
            VersionReq::Or { loc, left, right } => {
                let labels = vec![format!("||"), ns.loc_to_string(PathDisplay::FullPath, loc)];

                let node = self.add_node(Node::new("or", labels), Some(parent), Some(parent_rel));

                self.add_version(left, node, "left".into(), ns);
                self.add_version(right, node, "right".into(), ns);
            }
        }
    }
}

impl Namespace {
    pub fn dotgraphviz(&self) -> String {
        let mut dot = Dot {
            filename: format!("{}", self.files[self.top_file_no()].path.display()),
            nodes: Vec::new(),
            edges: Vec::new(),
        };

        // enums
        if !self.enums.is_empty() {
            let enums = dot.add_node(Node::new("enums", Vec::new()), None, None);

            for decl in &self.enums {
                let mut labels = decl
                    .values
                    .iter()
                    .map(|(name, _)| format!("value: {name}"))
                    .collect::<Vec<String>>();

                labels.insert(0, self.loc_to_string(PathDisplay::FullPath, &decl.loc));
                if let Some(contract) = &decl.contract {
                    labels.insert(0, format!("contract: {contract}"));
                }
                labels.insert(0, format!("name: {}", decl.id));

                let e = Node::new(&decl.id.name, labels);

                let node = dot.add_node(e, Some(enums), None);

                dot.add_tags(&decl.tags, node);
            }
        }

        // structs
        if !self.structs.is_empty() {
            let structs = dot.add_node(Node::new("structs", Vec::new()), None, None);

            for decl in &self.structs {
                if let pt::Loc::File(..) = &decl.loc {
                    let mut labels = vec![
                        format!("name:{}", decl.id),
                        self.loc_to_string(PathDisplay::FullPath, &decl.loc),
                    ];

                    if let Some(contract) = &decl.contract {
                        labels.insert(1, format!("contract: {contract}"));
                    }

                    for field in &decl.fields {
                        labels.push(format!(
                            "field name:{} ty:{}",
                            field.name_as_str(),
                            field.ty.to_string(self)
                        ));
                    }

                    let e = Node::new(&decl.id.name, labels);

                    let node = dot.add_node(e, Some(structs), None);

                    dot.add_tags(&decl.tags, node);
                }
            }
        }

        // events
        if !self.events.is_empty() {
            let events = dot.add_node(Node::new("events", Vec::new()), None, None);

            for decl in &self.events {
                let mut labels = vec![
                    format!("name:{}", decl.id),
                    self.loc_to_string(PathDisplay::FullPath, &decl.loc),
                ];

                if let Some(contract) = &decl.contract {
                    labels.insert(1, format!("contract: {contract}"));
                }

                if decl.anonymous {
                    labels.push(String::from("anonymous"));
                }

                for field in &decl.fields {
                    labels.push(format!(
                        "field name:{} ty:{} indexed:{}",
                        field.name_as_str(),
                        field.ty.to_string(self),
                        if field.indexed { "yes" } else { "no" }
                    ));
                }

                let e = Node::new(&decl.id.name, labels);

                let node = dot.add_node(e, Some(events), None);

                dot.add_tags(&decl.tags, node);
            }
        }

        // errors
        if !self.errors.is_empty() {
            let errors = dot.add_node(Node::new("errors", Vec::new()), None, None);

            for error in &self.errors {
                let mut labels = vec![
                    format!("name:{}", error.name),
                    self.loc_to_string(PathDisplay::FullPath, &error.loc),
                ];

                if let Some(contract) = &error.contract {
                    labels.insert(1, format!("contract: {contract}"));
                }

                for field in &error.fields {
                    labels.push(format!(
                        "field name:{} ty:{}",
                        field.name_as_str(),
                        field.ty.to_string(self),
                    ));
                }

                let e = Node::new(&error.name, labels);

                let node = dot.add_node(e, Some(errors), None);

                dot.add_tags(&error.tags, node);
            }
        }

        // user types
        if self.user_types.iter().any(|t| t.loc != pt::Loc::Builtin) {
            let types = dot.add_node(Node::new("types", Vec::new()), None, None);

            for decl in self.user_types.iter().filter(|t| t.loc != pt::Loc::Builtin) {
                let mut labels = vec![
                    format!("name:{} ty:{}", decl.name, decl.ty.to_string(self)),
                    self.loc_to_string(PathDisplay::FullPath, &decl.loc),
                ];

                if let Some(contract) = &decl.contract {
                    labels.insert(1, format!("contract: {contract}"));
                }

                let e = Node::new(&decl.name, labels);

                let node = dot.add_node(e, Some(types), None);

                dot.add_tags(&decl.tags, node);
            }
        }

        // free functions
        if self
            .functions
            .iter()
            .any(|func| func.contract_no.is_none() && func.loc_prototype != pt::Loc::Builtin)
        {
            let functions = dot.add_node(Node::new("free_functions", Vec::new()), None, None);

            for func in &self.functions {
                if func.contract_no.is_none() && func.loc_prototype != pt::Loc::Builtin {
                    dot.add_function(func, self, functions);
                }
            }
        }

        let contracts = dot.add_node(Node::new("contracts", Vec::new()), None, None);

        // contracts
        for c in &self.contracts {
            let contract = dot.add_node(
                Node::new(
                    "contract",
                    vec![
                        format!("contract {}", c.id),
                        self.loc_to_string(PathDisplay::FullPath, &c.loc),
                    ],
                ),
                Some(contracts),
                None,
            );

            dot.add_tags(&c.tags, contract);

            for base in &c.bases {
                let node = dot.add_node(
                    Node::new(
                        "base",
                        vec![
                            format!("base {}", self.contracts[base.contract_no].id),
                            self.loc_to_string(PathDisplay::FullPath, &base.loc),
                        ],
                    ),
                    Some(contract),
                    Some(String::from("base")),
                );

                if let Some((_, args)) = &base.constructor {
                    for (no, arg) in args.iter().enumerate() {
                        dot.add_expression(arg, None, self, node, format!("arg #{no}"));
                    }
                }
            }

            for var in &c.variables {
                let mut labels = vec![
                    format!("variable {}", var.name),
                    format!("visibility {}", var.visibility),
                    var.ty.to_string(self),
                    self.loc_to_string(PathDisplay::FullPath, &var.loc),
                ];

                if var.immutable {
                    labels.insert(2, String::from("immutable"));
                }

                if var.constant {
                    labels.insert(2, String::from("constant"));
                }

                let node = dot.add_node(
                    Node::new("var", labels),
                    Some(contract),
                    Some(String::from("variable")),
                );

                if let Some(initializer) = &var.initializer {
                    dot.add_expression(initializer, None, self, node, String::from("initializer"));
                }

                dot.add_tags(&var.tags, node);
            }

            for using in &c.using {
                let mut labels = match &using.list {
                    UsingList::Functions(functions) => functions
                        .iter()
                        .map(|using| {
                            let func = &self.functions[using.function_no];

                            let mut label = format!(
                                "function {} {}",
                                func.id,
                                self.loc_to_string(PathDisplay::FullPath, &func.loc_prototype)
                            );

                            if let Some(oper) = &using.oper {
                                label.push_str(&format!(" for operator {oper}"));
                            }
                            label
                        })
                        .collect(),
                    UsingList::Library(library_no) => {
                        let library = &self.contracts[*library_no];

                        vec![format!("library {}", library.id)]
                    }
                };

                if let Some(ty) = &using.ty {
                    labels.insert(0, format!("using for {}", ty.to_string(self)));
                }

                dot.add_node(
                    Node::new("using", labels),
                    Some(contract),
                    Some(String::from("base")),
                );
            }

            for func in &c.functions {
                dot.add_function(&self.functions[*func], self, contract);
            }
        }

        // pragmas
        if !self.pragmas.is_empty() {
            let pragmas = dot.add_node(Node::new("pragmas", Vec::new()), None, None);

            for pragma in &self.pragmas {
                match pragma {
                    Pragma::Identifier { loc, name, value } => {
                        let labels = vec![
                            format!("name: {}", name.name),
                            format!("value: {}", value.name),
                            self.loc_to_string(PathDisplay::FullPath, loc),
                        ];

                        dot.add_node(Node::new("pragma", labels), Some(pragmas), None);
                    }
                    Pragma::StringLiteral { loc, name, value } => {
                        let labels = vec![
                            format!("name: {}", name.name),
                            format!("value: '{}'", value.string),
                            self.loc_to_string(PathDisplay::FullPath, loc),
                        ];

                        dot.add_node(Node::new("pragma", labels), Some(pragmas), None);
                    }
                    Pragma::SolidityVersion { loc, versions } => {
                        let labels = vec![
                            "name: solidity".into(),
                            self.loc_to_string(PathDisplay::FullPath, loc),
                        ];

                        let node = dot.add_node(Node::new("pragma", labels), Some(pragmas), None);

                        for (no, version) in versions.iter().enumerate() {
                            dot.add_version(version, node, format!("version {no}"), self);
                        }
                    }
                }
            }
        }

        // diagnostics
        if !self.diagnostics.is_empty() {
            let diagnostics = dot.add_node(Node::new("diagnostics", Vec::new()), None, None);

            for diag in self.diagnostics.iter() {
                let mut labels = vec![
                    diag.message.to_string().replace('"', "\\\""),
                    format!("level {:?}", diag.level),
                ];

                labels.push(self.loc_to_string(PathDisplay::FullPath, &diag.loc));

                let node = dot.add_node(
                    Node::new("diagnostic", labels),
                    Some(diagnostics),
                    Some(format!("{:?}", diag.level)),
                );

                for note in &diag.notes {
                    dot.add_node(
                        Node::new(
                            "note",
                            vec![
                                note.message.to_string(),
                                self.loc_to_string(PathDisplay::FullPath, &note.loc),
                            ],
                        ),
                        Some(node),
                        Some(String::from("note")),
                    );
                }
            }
        }

        dot.write()
    }
}
