use indexmap::IndexMap;
use num_bigint::BigInt;
use std::collections::BTreeSet;

use crate::{
    parser::pt,
    sema::{ast::Type, symtable::Symtable},
};

#[derive(Clone)]
pub struct Variable {
    pub id: pt::Identifier,
    pub ty: Type,
    pub storage: Storage,
}

pub type Vars = IndexMap<usize, Variable>;

#[derive(Default)]
pub struct Vartable {
    pub vars: Vars,
    pub next_id: usize,
    pub dirty: Vec<DirtyTracker>,
}

pub struct DirtyTracker {
    lim: usize,
    set: BTreeSet<usize>,
}

#[derive(Clone)]
pub enum Storage {
    Constant(usize),
    Contract(BigInt),
    Local,
}

impl Vartable {
    pub fn from_symbol_table(sym: &Symtable, next_id: usize) -> Self {
        let mut vars = IndexMap::new();

        for (var_no, v) in &sym.vars {
            let id = Vartable::make_unique(&vars, &v.id, *var_no);

            vars.insert(
                *var_no,
                Variable {
                    id,
                    ty: v.ty.clone(),
                    storage: Storage::Local,
                },
            );
        }

        Vartable {
            vars,
            dirty: Vec::new(),
            next_id,
        }
    }

    pub fn add_symbol_table(&mut self, sym: &Symtable) {
        for (var_no, v) in &sym.vars {
            let id = Vartable::make_unique(&self.vars, &v.id, *var_no);

            self.vars.insert(
                *var_no,
                Variable {
                    id,
                    ty: v.ty.clone(),
                    storage: Storage::Local,
                },
            );
        }
    }

    fn make_unique(vars: &Vars, id: &pt::Identifier, no: usize) -> pt::Identifier {
        let mut id = id.clone();

        if id.name.is_empty() {
            id.name = format!("temp.{}", no);
        } else if vars.iter().any(|(_, var)| var.id.name == id.name) {
            id.name = format!("{}.{}", id.name, no);
        }

        id
    }

    pub fn new(next_id: usize) -> Self {
        Vartable {
            vars: IndexMap::new(),
            dirty: Vec::new(),
            next_id,
        }
    }

    pub fn temp_anonymous(&mut self, ty: &Type) -> usize {
        let var_no = self.next_id;
        self.next_id += 1;

        self.vars.insert(
            var_no,
            Variable {
                id: pt::Identifier {
                    name: format!("temp.{}", var_no),
                    loc: pt::Loc(0, 0, 0),
                },
                ty: ty.clone(),
                storage: Storage::Local,
            },
        );

        var_no
    }

    pub fn temp(&mut self, id: &pt::Identifier, ty: &Type) -> usize {
        let var_no = self.next_id;
        self.next_id += 1;

        self.vars.insert(
            var_no,
            Variable {
                id: pt::Identifier {
                    name: format!("{}.temp.{}", id.name, var_no),
                    loc: id.loc,
                },
                ty: ty.clone(),
                storage: Storage::Local,
            },
        );

        var_no
    }

    pub fn temp_name(&mut self, name: &str, ty: &Type) -> usize {
        let var_no = self.next_id;
        self.next_id += 1;

        self.vars.insert(
            var_no,
            Variable {
                id: pt::Identifier {
                    name: format!("{}.temp.{}", name, var_no),
                    loc: pt::Loc(0, 0, 0),
                },
                ty: ty.clone(),
                storage: Storage::Local,
            },
        );

        var_no
    }

    pub fn drain(self) -> (Vars, usize) {
        (self.vars, self.next_id)
    }

    // In order to create phi nodes, we need to track what vars are set in a certain scope
    pub fn set_dirty(&mut self, var_no: usize) {
        for e in &mut self.dirty {
            if var_no < e.lim {
                e.set.insert(var_no);
            }
        }
    }

    pub fn new_dirty_tracker(&mut self, lim: usize) {
        self.dirty.push(DirtyTracker {
            lim,
            set: BTreeSet::new(),
        });
    }

    pub fn pop_dirty_tracker(&mut self) -> BTreeSet<usize> {
        self.dirty.pop().unwrap().set
    }
}
