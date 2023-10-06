use indexmap::IndexMap;
use crate::codegen::vartable::Vars;
use crate::ssa_ir::expr::Operand;
use crate::ssa_ir::ssa_type::Type;

#[derive(Debug)]
pub struct Var {
    id: usize,
    ty: Type,
    name: String
}

#[derive(Debug)]
pub struct Vartable {
    pub vars: IndexMap<usize, Var>,
    pub next_id: usize,
}

impl Vartable {
    pub(crate) fn get_type(&self, id: &usize) -> Result<&Type, &'static str> {
        self.vars.get(id)
            .map(|var| &var.ty)
            .ok_or("Variable not found")
    }

    pub(crate) fn get_name(&self, id: &usize) -> Result<&str, &'static str> {
        self.vars.get(id)
            .map(|var| var.name.as_str())
            .ok_or("Variable not found")
    }

    pub(crate) fn get_operand(&self, id: &usize) -> Result<Operand, &'static str> {
        self.vars.get(id)
            .map(|var| Operand::Id {
                id: var.id,
                name: var.name.clone(),
            })
            .ok_or("Variable not found")
    }

    pub(crate) fn new_temp(&mut self, ty: Type) -> Operand {
        self.next_id += 1;

        let name = format!("temp.{}", self.next_id);
        let var = Var {
            id: self.next_id,
            ty: ty.clone(),
            name: name.clone(),
        };

        self.vars.insert(self.next_id, var);

        Operand::Id {
            id: self.next_id,
            name,
        }
    }
}

impl From<&Vars> for Vartable {
    fn from(value: &Vars) -> Self {
        let mut vars = IndexMap::new();
        let mut max_id = 0;
        for (id, var) in value {
            vars.insert(*id, Var {
                id: *id,
                ty: Type::try_from(&var.ty)?,
                name: var.id.name.clone()
            });
            if *id > max_id {
                max_id = *id;
            }
        }

        Vartable {
            vars,
            next_id: max_id + 1
        }
    }
}