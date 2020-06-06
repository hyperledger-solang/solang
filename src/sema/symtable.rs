use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::LinkedList;
use std::str;

use output::Output;
use parser::pt;
use sema::ast::{Namespace, Type};

#[derive(Clone)]
pub struct Variable {
    pub id: pt::Identifier,
    pub ty: Type,
    pub pos: usize,
}

struct VarScope(HashMap<String, usize>, Option<HashSet<usize>>);

#[derive(Default)]
pub struct Symtable {
    pub vars: Vec<Variable>,
    names: LinkedList<VarScope>,
    pub arguments: Vec<Option<usize>>,
    pub returns: Vec<usize>,
}

impl Symtable {
    pub fn new() -> Self {
        let mut list = LinkedList::new();
        list.push_front(VarScope(HashMap::new(), None));
        Symtable {
            vars: Vec::new(),
            names: list,
            arguments: Vec::new(),
            returns: Vec::new(),
        }
    }

    pub fn add(&mut self, id: &pt::Identifier, ty: Type, ns: &mut Namespace) -> Option<usize> {
        let pos = self.vars.len();

        self.vars.push(Variable {
            id: id.clone(),
            ty,
            pos,
        });

        // the variable has no name, like unnamed return or parameters values
        if !id.name.is_empty() {
            if let Some(ref prev) = self.find(&id.name) {
                ns.diagnostics.push(Output::error_with_note(
                    id.loc,
                    format!("{} is already declared", id.name.to_string()),
                    prev.id.loc,
                    "location of previous declaration".to_string(),
                ));
                return None;
            }

            self.names
                .front_mut()
                .unwrap()
                .0
                .insert(id.name.to_string(), pos);
        }

        Some(pos)
    }

    pub fn find(&self, name: &str) -> Option<&Variable> {
        for scope in &self.names {
            if let Some(n) = scope.0.get(name) {
                return Some(&self.vars[*n]);
            }
        }

        None
    }

    pub fn new_scope(&mut self) {
        self.names.push_front(VarScope(HashMap::new(), None));
    }

    pub fn leave_scope(&mut self) {
        self.names.pop_front();
    }

    pub fn get_name(&self, pos: usize) -> &str {
        &self.vars[pos].id.name
    }
}

pub struct LoopScope {
    pub no_breaks: usize,
    pub no_continues: usize,
}

pub struct LoopScopes(LinkedList<LoopScope>);

impl LoopScopes {
    pub fn new() -> Self {
        LoopScopes(LinkedList::new())
    }

    pub fn new_scope(&mut self) {
        self.0.push_front(LoopScope {
            no_breaks: 0,
            no_continues: 0,
        })
    }

    pub fn leave_scope(&mut self) -> LoopScope {
        self.0.pop_front().unwrap()
    }

    pub fn do_break(&mut self) -> bool {
        match self.0.front_mut() {
            Some(scope) => {
                scope.no_breaks += 1;
                true
            }
            None => false,
        }
    }

    pub fn do_continue(&mut self) -> bool {
        match self.0.front_mut() {
            Some(scope) => {
                scope.no_continues += 1;
                true
            }
            None => false,
        }
    }
}
