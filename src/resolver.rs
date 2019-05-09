use ast;
use cfg;
use output::{Output,Note};
use std::collections::HashMap;
use serde::Serialize;


#[derive(Serialize)]
pub struct ABIParam {
    pub name: String,
    #[serde(rename="type")]
    pub ty: String,
}

#[derive(Serialize)]
pub struct ABI {
    pub name: String,
    #[serde(rename="type")]
    pub ty: String,
    pub inputs: Vec<ABIParam>,
    pub outputs: Vec<ABIParam>
}

#[derive(PartialEq,Clone)]
pub enum TypeName {
    Elementary(ast::ElementaryTypeName),
    Enum(usize),
}

impl TypeName {
    pub fn to_string(&self, ns: &Contract) -> String {
        match self {
            TypeName::Elementary(e) => e.to_string(),
            TypeName::Enum(n) => format!("enum {}", ns.enums[*n].name)
        }
    }

    pub fn bits(&self) -> u16 {
       match self {
            TypeName::Elementary(e) => e.bits(),
            _ => panic!("type not allowed")
        }
    }

    pub fn signed(&self) -> bool {
       match self {
            TypeName::Elementary(e) => e.signed(),
            TypeName::Enum(_) => false
        }
    }

    pub fn ordered(&self) -> bool {
       match self {
            TypeName::Elementary(e) => e.ordered(),
            TypeName::Enum(_) => false
        }
    }

    pub fn new_bool() -> Self {
        TypeName::Elementary(ast::ElementaryTypeName::Bool)
    }
}

pub struct EnumDecl {
    pub name: String,
    pub ty: ast::ElementaryTypeName,
    pub values: HashMap<String, (ast::Loc, usize)>,
}

pub struct Parameter {
    pub name: String,
    pub ty: TypeName,
}

impl Parameter {
    fn to_abi(&self, ns: &Contract) -> ABIParam {
        ABIParam{
            name: self.name.to_string(),
            ty: match &self.ty {
                TypeName::Elementary(e) => e.to_string(),
                TypeName::Enum(ref i) => ns.enums[*i].ty.to_string()
            }
        }
    }
}

pub struct FunctionDecl {
    pub loc: ast::Loc,
    pub constructor: bool,
    pub name: Option<String>,
    pub sig: String,
    pub ast_index: usize,
    pub params: Vec<Parameter>,
    pub returns: Vec<Parameter>,
    pub cfg: Option<Box<cfg::ControlFlowGraph>>,
}

pub struct ContractVariable {
    pub name: String,
    pub ty: TypeName,
    pub storage: Option<usize>,
}

pub enum Symbol {
    Enum(ast::Loc, usize),
    Function(Vec<(ast::Loc, usize)>),
    Variable(ast::Loc, usize),
}

pub struct Contract {
    pub name: String,
    pub enums: Vec<EnumDecl>,
    // structs/events
    pub functions: Vec<FunctionDecl>,
    pub variables: Vec<ContractVariable>,
    top_of_contract_storage: usize,
    symbols: HashMap<String, Symbol>,
}

impl Contract {
    fn add_symbol(&mut self, id: &ast::Identifier, symbol: Symbol, errors: &mut Vec<Output>) -> bool {
        if let Some(prev) = self.symbols.get(&id.name) {
            match prev {
                Symbol::Enum(e, _) => {
                    errors.push(Output::error_with_note(id.loc, format!("{} is already defined as enum", id.name.to_string()),
                            e.clone(), "location of previous definition".to_string()));
                },
                Symbol::Function(v) => {
                    let mut notes = Vec::new();

                    for e in v {
                        notes.push(Note{pos: e.0.clone(), message: "location of previous definition".into()});
                    }

                    errors.push(Output::error_with_notes(id.loc, format!("{} is already defined as function", id.name.to_string()),
                            notes));
                },
                Symbol::Variable(e, _) => {
                    errors.push(Output::error_with_note(id.loc, format!("{} is already defined as state variable", id.name.to_string()),
                            e.clone(), "location of previous definition".to_string()));
                }
            }
            return false;
        }

        self.symbols.insert(id.name.to_string(), symbol);

        true
    }

    pub fn resolve(&self, id: &ast::TypeName, errors: &mut Vec<Output>) -> Option<TypeName> {
        match id {
            ast::TypeName::Elementary(e) => Some(TypeName::Elementary(*e)),
            ast::TypeName::Unresolved(s) => {
                match self.symbols.get(&s.name) {
                    None => {
                        errors.push(Output::decl_error(s.loc, format!("`{}' is not declared", s.name)));
                        None
                    },
                    Some(Symbol::Enum(_, n)) => {
                        Some(TypeName::Enum(*n))
                    }
                    Some(Symbol::Function(_)) => {
                        errors.push(Output::decl_error(s.loc, format!("`{}' is a function", s.name)));
                        None
                    }
                    Some(Symbol::Variable(_, n)) => {
                        Some(self.variables[*n].ty.clone())
                    }
                }
            }
        }
    }

    pub fn check_shadowing(&self, id: &ast::Identifier, errors: &mut Vec<Output>) {
        match self.symbols.get(&id.name) {
            Some(Symbol::Enum(loc, _)) => {
                errors.push(Output::warning_with_note(id.loc, format!("declaration of `{}' shadows enum", id.name),
                        loc.clone(), format!("previous declaration of enum")));
            },
            Some(Symbol::Function(v)) => {
                let notes = v.iter().map(|(pos, _)| Note{pos: pos.clone(), message: "previous declaration of function".to_owned()}).collect();
                errors.push(Output::warning_with_notes(id.loc, format!("declaration of `{}' shadows function", id.name), notes));
            },
            Some(Symbol::Variable(loc, _)) => {
                errors.push(Output::warning_with_note(id.loc, format!("declaration of `{}' shadows state variable", id.name),
                        loc.clone(), format!("previous declaration of state variable")));
            },
            None => {}
        }
    }

    pub fn fallback_function(&self) -> Option<usize> {
        for (i, f) in self.functions.iter().enumerate() {
            if !f.constructor && None == f.name {
                return Some(i);
            }
        }
        return None;
    }

    pub fn constructor_function(&self) -> Option<usize> {
        for (i, f) in self.functions.iter().enumerate() {
            if f.constructor {
                return Some(i);
            }
        }
        return None;
    }

    pub fn generate_abi(&self) -> Vec<ABI> {
        let mut abis = Vec::new();

        for f in &self.functions {
            let (ty, name) = if f.constructor {
                ("constructor".to_string(), "".to_string())
            } else {
                match &f.name {
                    Some(n) => ("function".to_string(), n.to_string()),
                    None => ("fallback".to_string(), "".to_string()),
                }
            };

            abis.push(ABI{
                name,
                ty,
                inputs: f.params.iter().map(|p| p.to_abi(&self)).collect(),
                outputs: f.returns.iter().map(|p| p.to_abi(&self)).collect(),
            })
        }

        abis
    }

    pub fn to_string(&self) -> String {
        let mut s = String::new();

        for f in &self.functions {
            if let Some(ref name) = f.name {
                s.push_str(&format!("# function {}\n", name));
            } else {
                s.push_str(&format!("# constructor\n"));
            }

            if let Some(ref cfg) = f.cfg {
                s.push_str(&cfg.to_string(self));
            }
        }

        s
    }
}

pub fn resolver(s: ast::SourceUnit) -> (Vec<Contract>, Vec<Output>) {
    let mut namespace = Vec::new();
    let mut errors = Vec::new();

    for part in s.parts {
        if let ast::SourceUnitPart::ContractDefinition(def) = part {
            if let Some(c) = resolve_contract(def, &mut errors) {
                namespace.push(c)
            }
        }
    }

    (namespace, errors)
}

fn resolve_contract(def: Box<ast::ContractDefinition>, errors: &mut Vec<Output>) -> Option<Contract> {
    let mut ns = Contract{
        name: def.name.name.to_string(),
        enums: Vec::new(),
        functions: Vec::new(),
        variables: Vec::new(),
        top_of_contract_storage: 0,
        symbols: HashMap::new(),
    };

    errors.push(Output::info(def.loc, format!("found contract {}", def.name.name)));

    let mut broken = false;

    // first resolve enums
    for parts in &def.parts {
        if let ast::ContractPart::EnumDefinition(ref e) = parts {
            let pos = ns.enums.len();

            ns.enums.push(enum_decl(e, errors));

            if !ns.add_symbol(&e.name, Symbol::Enum(e.name.loc, pos), errors) {
                broken = true;
            }
        }
    }

    // FIXME: next resolve structs/event

    // resolve state variables
    for parts in &def.parts {
        if let ast::ContractPart::ContractVariableDefinition(ref s) = parts {
            if !var_decl(s, &mut ns, errors) {
                broken = true;
            }
        }
    }

    // resolve function signatures
    for (i, parts) in def.parts.iter().enumerate() {
        if let ast::ContractPart::FunctionDefinition(ref f) = parts {
            if !func_decl(f, i, &mut ns, errors) {
                broken = true;
            }
        }
    }

    // resolve function bodies
    for f in 0..ns.functions.len() {
        let ast_index = ns.functions[f].ast_index;
        if let ast::ContractPart::FunctionDefinition(ref ast_f) = def.parts[ast_index] {
            match cfg::generate_cfg(ast_f, &ns.functions[f], &ns, errors) {
                Ok(c) => ns.functions[f].cfg = Some(c),
                Err(_) => broken = true
            }
        }
    }

    if !broken {
        Some(ns)
    } else {
        None
    }
}

fn enum_decl(enum_: &ast::EnumDefinition, errors: &mut Vec<Output>) -> EnumDecl {
    // Number of bits required to represent this enum
    let mut bits = std::mem::size_of::<usize>() as u32 * 8 - (enum_.values.len() - 1).leading_zeros();
    // round it up to the next
    if bits <= 8 {
        bits = 8;
    } else {
        bits += 7;
        bits -= bits % 8;
    }

    // check for duplicates
    let mut entries: HashMap<String, (ast::Loc, usize)> = HashMap::new();

    for (i, e) in enum_.values.iter().enumerate() {
        if let Some(prev) = entries.get(&e.name.to_string()) {
            errors.push(Output::error_with_note(e.loc, format!("duplicate enum value {}", e.name),
                prev.0.clone(), "location of previous definition".to_string()));
            continue;
        }
        
        entries.insert(e.name.to_string(), (e.loc, i));
    }

    EnumDecl{
        name: enum_.name.name.to_string(),
        ty: ast::ElementaryTypeName::Uint(bits as u16),
        values: entries
    }
}

#[test]
fn enum_256values_is_uint8() {
    let mut e = ast::EnumDefinition{
        name: ast::Identifier{loc: ast::Loc(0, 0), name: "foo".into()},
        values: Vec::new(),
    };

    e.values.push(ast::Identifier{loc: ast::Loc(0, 0), name: "first".into()});

    let f = enum_decl(&e, &mut Vec::new());
    assert_eq!(f.ty, ast::ElementaryTypeName::Uint(8));

    for i in 1..256 {
        e.values.push(ast::Identifier{loc: ast::Loc(0, 0), name: format!("val{}", i)})
    }

    assert_eq!(e.values.len(), 256);

    let r = enum_decl(&e, &mut Vec::new());
    assert_eq!(r.ty, ast::ElementaryTypeName::Uint(8));

    e.values.push(ast::Identifier{loc: ast::Loc(0, 0), name: "another".into()});

    let r2 = enum_decl(&e, &mut Vec::new());
    assert_eq!(r2.ty, ast::ElementaryTypeName::Uint(16));
}

fn var_decl(s: &ast::ContractVariableDefinition, ns: &mut Contract, errors: &mut Vec<Output>) -> bool {
    let ty = match ns.resolve(&s.ty, errors) {
        Some(s) => s,
        None => {
            return false;
        }
    };

    let mut is_constant = false;

    for attr in &s.attrs {
        match attr {
            ast::VariableAttribute::Constant(loc) => {
                if is_constant {
                    errors.push(Output::warning(loc.clone(), format!("duplicate constant attribute")));
                }
                is_constant = true;
            },
            _ => ()
        }
    }

    let storage = if !is_constant  {
        ns.top_of_contract_storage += 1;
        Some(ns.top_of_contract_storage)
    } else {
        None
    };

    let sdecl = ContractVariable{
        name: s.name.name.to_string(),
        storage: storage,
        ty
    };

    // FIXME: resolve init expression and check for constant (if constant)
    // init expression can call functions and access other state variables

    let pos = ns.variables.len();

    ns.variables.push(sdecl);

    ns.add_symbol(&s.name, Symbol::Variable(s.loc, pos), errors)
}

fn func_decl(f: &ast::FunctionDefinition, i: usize, ns: &mut Contract, errors: &mut Vec<Output>) -> bool {
    let mut params = Vec::new();
    let mut returns = Vec::new();
    let mut success = true;

    if f.constructor && !f.returns.is_empty() {
        errors.push(Output::warning(f.loc, format!("constructor cannot have return values")));
        return false;
    } else if !f.constructor && f.name == None {
        if !f.returns.is_empty() {
            errors.push(Output::warning(f.loc, format!("fallback function cannot have return values")));
            success = false;
        }

        if !f.params.is_empty() {
            errors.push(Output::warning(f.loc, format!("fallback function cannot have parameters")));
            success = false;
        }
    }

    for p in &f.params {
        match ns.resolve(&p.typ, errors) {
            Some(s) => params.push(Parameter{
                name: p.name.as_ref().map_or("".to_string(), |id| id.name.to_string()),
                ty: s
            }),
            None => { success = false },
        }
    }

    for r in &f.returns {
        if let Some(ref n) = r.name {
            errors.push(Output::warning(n.loc, format!("named return value `{}' not allowed", n.name)));
        }

        match ns.resolve(&r.typ, errors) {
            Some(s) => returns.push(Parameter{
                name: r.name.as_ref().map_or("".to_string(), |id| id.name.to_string()),
                ty: s
            }),
            None => { success = false },
        }
    }

    if !success {
        return false;
    }

    let name = match f.name {
        Some(ref n) => Some(n.name.to_string()),
        None => None,
    };

    let fdecl = FunctionDecl{
        loc: f.loc,
        sig: external_signature(&name, &params, &ns),
        name: name,
        constructor: f.constructor,
        ast_index: i,
        params,
        returns,
        cfg: None
    };

    if f.constructor {
        // fallback function
        if let Some(i) = ns.constructor_function() {
            let prev = &ns.functions[i];
            errors.push(Output::error_with_note(f.loc, "constructor already defined".to_string(),
                    prev.loc, "location of previous definition".to_string()));
            return false;
        }

        ns.functions.push(fdecl);

        true
    } else if let Some(ref id) = f.name {
        if let Some(Symbol::Function(ref mut v)) = ns.symbols.get_mut(&id.name) {
            // check if signature already present
            for o in v.iter() {
                if fdecl.sig == ns.functions[o.1].sig {
                    errors.push(Output::error_with_note(f.loc, "overloaded function with this signature already exist".to_string(),
                            o.0.clone(), "location of previous definition".to_string()));
                    return false;
                }
            }

            let pos = ns.functions.len();

            ns.functions.push(fdecl);

            v.push((f.loc, pos));
            return true;
        }

        let pos = ns.functions.len();

        ns.functions.push(fdecl);

        ns.add_symbol(id, Symbol::Function(vec!((id.loc, pos))), errors)
    } else {
        // fallback function
        if let Some(i) = ns.fallback_function() {
            let prev = &ns.functions[i];
            errors.push(Output::error_with_note(f.loc, "fallback function already defined".to_string(),
                    prev.loc, "location of previous definition".to_string()));
            return false;
        }

        ns.functions.push(fdecl);

        true
    }
}

pub fn external_signature(name: &Option<String>, params: &Vec<Parameter>, ns: &Contract) -> String {
    let mut sig = match name { Some(ref n) => n.to_string(), None => "".to_string() };

    sig.push('(');

    for (i, p) in params.iter().enumerate() {
        if i > 0 {
            sig.push(',');
        }

        sig.push_str(&match &p.ty {
            TypeName::Elementary(e) => e.to_string(),
            TypeName::Enum(i) => ns.enums[*i].ty.to_string()
        });
    }

    sig.push(')');

    sig
}

#[test]
fn signatures() {
    let ns = Contract{
        name: String::from("foo"),
        enums: Vec::new(),
        functions: Vec::new(),
        variables: Vec::new(),
        top_of_contract_storage: 0,
        symbols: HashMap::new(),
    };

    assert_eq!(external_signature(&Some("foo".to_string()), &vec!(
        Parameter{name: "".to_string(), ty: TypeName::Elementary(ast::ElementaryTypeName::Uint(8))},
        Parameter{name: "".to_string(), ty: TypeName::Elementary(ast::ElementaryTypeName::Address)},
        ),
        &ns),
        "foo(uint8,address)");
}
