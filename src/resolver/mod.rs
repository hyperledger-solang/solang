use abi;
use emit;
use output::{Note, Output};
use parser::ast;
use std::collections::HashMap;
use tiny_keccak::keccak256;
use Target;

mod address;
mod builtin;
pub mod cfg;
mod functions;
mod variables;

use resolver::cfg::{ControlFlowGraph, Instr, Vartable};

#[derive(PartialEq, Clone, Debug)]
pub enum Type {
    Primitive(ast::PrimitiveType),
    FixedArray(Box<Type>, Vec<usize>),
    Enum(usize),
    Noreturn,
}

impl Type {
    pub fn to_string(&self, ns: &Contract) -> String {
        match self {
            Type::Primitive(e) => e.to_string(),
            Type::Enum(n) => format!("enum {}.{}", ns.name, ns.enums[*n].name),
            Type::FixedArray(ty, len) => format!(
                "{}[{}]",
                ty.to_string(ns),
                len.iter().map(|l| format!("[{}]", l)).collect::<String>()
            ),
            Type::Noreturn => "no return".to_owned(),
        }
    }

    pub fn to_primitive_string(&self, ns: &Contract) -> String {
        match self {
            Type::Primitive(e) => e.to_string(),
            Type::Enum(n) => ns.enums[*n].ty.to_string(),
            Type::FixedArray(ty, len) => format!(
                "{}{}",
                ty.to_primitive_string(ns),
                len.iter().map(|l| format!("[{}]", l)).collect::<String>()
            ),
            Type::Noreturn => "no return".to_owned(),
        }
    }

    pub fn bits(&self) -> u16 {
        match self {
            Type::Primitive(e) => e.bits(),
            _ => panic!("type not allowed"),
        }
    }

    pub fn signed(&self) -> bool {
        match self {
            Type::Primitive(e) => e.signed(),
            Type::Enum(_) => false,
            Type::FixedArray(_, _) => unreachable!(),
            Type::Noreturn => unreachable!(),
        }
    }

    pub fn ordered(&self) -> bool {
        match self {
            Type::Primitive(e) => e.ordered(),
            Type::Enum(_) => false,
            Type::FixedArray(_, _) => unreachable!(),
            Type::Noreturn => unreachable!(),
        }
    }

    pub fn new_bool() -> Self {
        Type::Primitive(ast::PrimitiveType::Bool)
    }
}

pub struct EnumDecl {
    pub name: String,
    pub ty: ast::PrimitiveType,
    pub values: HashMap<String, (ast::Loc, usize)>,
}

pub struct Parameter {
    pub name: String,
    pub ty: Type,
}

pub struct FunctionDecl {
    pub doc: Vec<String>,
    pub loc: ast::Loc,
    pub name: String,
    pub fallback: bool,
    pub signature: String,
    pub ast_index: Option<usize>,
    pub mutability: Option<ast::StateMutability>,
    pub visibility: ast::Visibility,
    pub params: Vec<Parameter>,
    pub returns: Vec<Parameter>,
    pub wasm_return: bool,
    pub cfg: Option<Box<cfg::ControlFlowGraph>>,
}

impl FunctionDecl {
    fn new(
        loc: ast::Loc,
        name: String,
        doc: Vec<String>,
        fallback: bool,
        ast_index: Option<usize>,
        mutability: Option<ast::StateMutability>,
        visibility: ast::Visibility,
        params: Vec<Parameter>,
        returns: Vec<Parameter>,
        ns: &Contract,
    ) -> Self {
        let mut signature = name.to_owned();

        signature.push('(');

        for (i, p) in params.iter().enumerate() {
            if i > 0 {
                signature.push(',');
            }

            signature.push_str(&p.ty.to_string(ns));
        }

        signature.push(')');

        let wasm_return = returns.len() == 1 && !returns[0].ty.stack_based();

        FunctionDecl {
            doc,
            loc,
            name,
            fallback,
            signature,
            ast_index,
            mutability,
            visibility,
            params,
            returns,
            wasm_return,
            cfg: None,
        }
    }

    pub fn selector(&self) -> u32 {
        let res = keccak256(self.signature.as_bytes());

        u32::from_le_bytes([res[0], res[1], res[2], res[3]])
    }

    pub fn wasm_symbol(&self, ns: &Contract) -> String {
        let mut sig = self.name.to_owned();

        if !self.params.is_empty() {
            sig.push_str("__");

            for (i, p) in self.params.iter().enumerate() {
                if i > 0 {
                    sig.push('_');
                }

                sig.push_str(&match &p.ty {
                    Type::Primitive(e) => e.to_string(),
                    Type::Enum(i) => ns.enums[*i].name.to_owned(),
                    Type::FixedArray(ty, len) => format!(
                        "{}{}",
                        ty.to_string(ns),
                        len.iter().map(|r| format!(":{}", r)).collect::<String>()
                    ),
                    Type::Noreturn => unreachable!(),
                });
            }
        }

        sig
    }
}

pub enum ContractVariableType {
    Storage(usize),
    Constant(usize),
}

pub struct ContractVariable {
    pub doc: Vec<String>,
    pub name: String,
    pub ty: Type,
    pub visibility: ast::Visibility,
    pub var: ContractVariableType,
}

impl ContractVariable {
    pub fn is_storage(&self) -> bool {
        if let ContractVariableType::Storage(_) = self.var {
            true
        } else {
            false
        }
    }
}

pub enum Symbol {
    Enum(ast::Loc, usize),
    Function(Vec<(ast::Loc, usize)>),
    Variable(ast::Loc, usize),
}

pub struct Contract {
    pub doc: Vec<String>,
    pub name: String,
    pub enums: Vec<EnumDecl>,
    // structs/events
    pub constructors: Vec<FunctionDecl>,
    pub functions: Vec<FunctionDecl>,
    pub variables: Vec<ContractVariable>,
    pub constants: Vec<cfg::Expression>,
    pub initializer: cfg::ControlFlowGraph,
    pub target: Target,
    top_of_contract_storage: usize,
    symbols: HashMap<String, Symbol>,
}

impl Contract {
    fn add_symbol(
        &mut self,
        id: &ast::Identifier,
        symbol: Symbol,
        errors: &mut Vec<Output>,
    ) -> bool {
        if let Some(prev) = self.symbols.get(&id.name) {
            match prev {
                Symbol::Enum(e, _) => {
                    errors.push(Output::error_with_note(
                        id.loc,
                        format!("{} is already defined as enum", id.name.to_string()),
                        e.clone(),
                        "location of previous definition".to_string(),
                    ));
                }
                Symbol::Function(v) => {
                    let mut notes = Vec::new();

                    for e in v {
                        notes.push(Note {
                            pos: e.0.clone(),
                            message: "location of previous definition".into(),
                        });
                    }

                    errors.push(Output::error_with_notes(
                        id.loc,
                        format!("{} is already defined as function", id.name.to_string()),
                        notes,
                    ));
                }
                Symbol::Variable(e, _) => {
                    errors.push(Output::error_with_note(
                        id.loc,
                        format!(
                            "{} is already defined as state variable",
                            id.name.to_string()
                        ),
                        e.clone(),
                        "location of previous definition".to_string(),
                    ));
                }
            }
            return false;
        }

        self.symbols.insert(id.name.to_string(), symbol);

        true
    }

    pub fn resolve_type(&self, id: &ast::Type, errors: &mut Vec<Output>) -> Result<Type, ()> {
        match id {
            ast::Type::Primitive(e) => Ok(Type::Primitive(*e)),
            ast::Type::Unresolved(s) => match self.symbols.get(&s.name) {
                None => {
                    errors.push(Output::decl_error(
                        s.loc,
                        format!("`{}' is not declared", s.name),
                    ));
                    Err(())
                }
                Some(Symbol::Enum(_, n)) => Ok(Type::Enum(*n)),
                Some(Symbol::Function(_)) => {
                    errors.push(Output::decl_error(
                        s.loc,
                        format!("`{}' is a function", s.name),
                    ));
                    Err(())
                }
                Some(Symbol::Variable(_, n)) => Ok(self.variables[*n].ty.clone()),
            },
        }
    }

    pub fn resolve_enum(&self, id: &ast::Identifier) -> Option<usize> {
        match self.symbols.get(&id.name) {
            Some(Symbol::Enum(_, n)) => Some(*n),
            _ => None,
        }
    }

    pub fn resolve_func(
        &self,
        id: &ast::Identifier,
        errors: &mut Vec<Output>,
    ) -> Result<&Vec<(ast::Loc, usize)>, ()> {
        match self.symbols.get(&id.name) {
            Some(Symbol::Function(v)) => Ok(v),
            _ => {
                errors.push(Output::error(
                    id.loc.clone(),
                    format!("unknown function or type"),
                ));

                Err(())
            }
        }
    }

    pub fn resolve_var(&self, id: &ast::Identifier, errors: &mut Vec<Output>) -> Result<usize, ()> {
        match self.symbols.get(&id.name) {
            None => {
                errors.push(Output::decl_error(
                    id.loc.clone(),
                    format!("`{}' is not declared", id.name),
                ));
                Err(())
            }
            Some(Symbol::Enum(_, _)) => {
                errors.push(Output::decl_error(
                    id.loc.clone(),
                    format!("`{}' is an enum", id.name),
                ));
                Err(())
            }
            Some(Symbol::Function(_)) => {
                errors.push(Output::decl_error(
                    id.loc.clone(),
                    format!("`{}' is a function", id.name),
                ));
                Err(())
            }
            Some(Symbol::Variable(_, n)) => Ok(*n),
        }
    }

    pub fn check_shadowing(&self, id: &ast::Identifier, errors: &mut Vec<Output>) {
        match self.symbols.get(&id.name) {
            Some(Symbol::Enum(loc, _)) => {
                errors.push(Output::warning_with_note(
                    id.loc,
                    format!("declaration of `{}' shadows enum", id.name),
                    loc.clone(),
                    format!("previous declaration of enum"),
                ));
            }
            Some(Symbol::Function(v)) => {
                let notes = v
                    .iter()
                    .map(|(pos, _)| Note {
                        pos: pos.clone(),
                        message: "previous declaration of function".to_owned(),
                    })
                    .collect();
                errors.push(Output::warning_with_notes(
                    id.loc,
                    format!("declaration of `{}' shadows function", id.name),
                    notes,
                ));
            }
            Some(Symbol::Variable(loc, _)) => {
                errors.push(Output::warning_with_note(
                    id.loc,
                    format!("declaration of `{}' shadows state variable", id.name),
                    loc.clone(),
                    format!("previous declaration of state variable"),
                ));
            }
            None => {}
        }
    }

    pub fn fallback_function(&self) -> Option<usize> {
        for (i, f) in self.functions.iter().enumerate() {
            if f.fallback {
                return Some(i);
            }
        }
        return None;
    }

    pub fn to_string(&self) -> String {
        let mut s = format!("#\n# Contract: {}\n#\n\n", self.name);

        s.push_str("# storage initializer\n");
        s.push_str(&self.initializer.to_string(self));
        s.push_str("\n");

        for f in &self.constructors {
            s.push_str(&format!("# constructor {}\n", f.signature));

            if let Some(ref cfg) = f.cfg {
                s.push_str(&cfg.to_string(self));
            }
        }

        for (i, f) in self.functions.iter().enumerate() {
            if f.name != "" {
                s.push_str(&format!("# function({}) {}\n", i, f.signature));
            } else {
                s.push_str(&format!("# fallback({})\n", i));
            }

            if let Some(ref cfg) = f.cfg {
                s.push_str(&cfg.to_string(self));
            }
        }

        s
    }

    pub fn abi(&self, verbose: bool) -> (String, &'static str) {
        abi::generate_abi(self, verbose)
    }

    pub fn emit<'a>(
        &'a self,
        context: &'a inkwell::context::Context,
        filename: &'a str,
        opt: &str,
    ) -> emit::Contract {
        emit::Contract::build(context, self, filename, opt)
    }
}

pub fn resolver(s: ast::SourceUnit, target: &Target) -> (Vec<Contract>, Vec<Output>) {
    let mut contracts = Vec::new();
    let mut errors = Vec::new();

    for part in s.0 {
        match part {
            ast::SourceUnitPart::ContractDefinition(def) => {
                if let Some(c) = resolve_contract(def, &target, &mut errors) {
                    contracts.push(c)
                }
            }
            ast::SourceUnitPart::PragmaDirective(name, _) => {
                if name.name == "solidity" {
                    errors.push(Output::info(
                        name.loc.clone(),
                        format!("pragma solidity is ignored"),
                    ));
                } else {
                    errors.push(Output::warning(
                        name.loc.clone(),
                        format!("unknown pragma {} ignored", name.name),
                    ));
                }
            }
            _ => unimplemented!(),
        }
    }

    (contracts, errors)
}

fn resolve_contract(
    def: Box<ast::ContractDefinition>,
    target: &Target,
    errors: &mut Vec<Output>,
) -> Option<Contract> {
    let mut ns = Contract {
        name: def.name.name.to_string(),
        doc: def.doc.clone(),
        enums: Vec::new(),
        constructors: Vec::new(),
        functions: Vec::new(),
        variables: Vec::new(),
        constants: Vec::new(),
        initializer: cfg::ControlFlowGraph::new(),
        target: target.clone(),
        top_of_contract_storage: 0,
        symbols: HashMap::new(),
    };

    errors.push(Output::info(
        def.loc,
        format!("found contract {}", def.name.name),
    ));

    builtin::add_builtin_function(&mut ns);

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

    // resolve function signatures
    for (i, parts) in def.parts.iter().enumerate() {
        if let ast::ContractPart::FunctionDefinition(ref f) = parts {
            if !functions::function_decl(f, i, &mut ns, errors) {
                broken = true;
            }
        }
    }

    // resolve state variables
    if variables::contract_variables(&def, &mut ns, errors) {
        broken = true;
    }

    // resolve constructor bodies
    for f in 0..ns.constructors.len() {
        if let Some(ast_index) = ns.constructors[f].ast_index {
            if let ast::ContractPart::FunctionDefinition(ref ast_f) = def.parts[ast_index] {
                match cfg::generate_cfg(ast_f, &ns.constructors[f], &ns, errors) {
                    Ok(c) => ns.constructors[f].cfg = Some(c),
                    Err(_) => broken = true,
                }
            }
        }
    }

    // Substrate requires one constructor
    if ns.constructors.is_empty() && target == &Target::Substrate {
        let mut fdecl = FunctionDecl::new(
            ast::Loc(0, 0),
            "".to_owned(),
            vec![],
            false,
            None,
            None,
            ast::Visibility::Public(ast::Loc(0, 0)),
            Vec::new(),
            Vec::new(),
            &ns,
        );

        let mut vartab = Vartable::new();
        let mut cfg = ControlFlowGraph::new();

        cfg.add(&mut vartab, Instr::Return { value: Vec::new() });
        cfg.vars = vartab.drain();

        fdecl.cfg = Some(Box::new(cfg));

        ns.constructors.push(fdecl);
    }

    // resolve function bodies
    for f in 0..ns.functions.len() {
        if let Some(ast_index) = ns.functions[f].ast_index {
            if let ast::ContractPart::FunctionDefinition(ref ast_f) = def.parts[ast_index] {
                match cfg::generate_cfg(ast_f, &ns.functions[f], &ns, errors) {
                    Ok(c) => {
                        match &ns.functions[f].mutability {
                            Some(ast::StateMutability::Pure(loc)) => {
                                if c.writes_contract_storage {
                                    errors.push(Output::error(
                                        loc.clone(),
                                        format!(
                                            "function declared pure but writes contract storage"
                                        ),
                                    ));
                                    broken = true;
                                } else if c.reads_contract_storage {
                                    errors.push(Output::error(
                                        loc.clone(),
                                        format!(
                                            "function declared pure but reads contract storage"
                                        ),
                                    ));
                                    broken = true;
                                }
                            }
                            Some(ast::StateMutability::View(loc)) => {
                                if c.writes_contract_storage {
                                    errors.push(Output::error(
                                        loc.clone(),
                                        format!(
                                            "function declared view but writes contract storage"
                                        ),
                                    ));
                                    broken = true;
                                } else if !c.reads_contract_storage {
                                    errors.push(Output::warning(
                                        loc.clone(),
                                        format!("function can be declared pure"),
                                    ));
                                }
                            }
                            Some(ast::StateMutability::Payable(_)) => {
                                unimplemented!();
                            }
                            None => {
                                let loc = &ns.functions[f].loc;

                                if !c.writes_contract_storage && !c.reads_contract_storage {
                                    errors.push(Output::warning(
                                        loc.clone(),
                                        format!("function can be declare pure"),
                                    ));
                                } else if !c.writes_contract_storage {
                                    errors.push(Output::warning(
                                        loc.clone(),
                                        format!("function can be declared view"),
                                    ));
                                }
                            }
                        }
                        ns.functions[f].cfg = Some(c);
                    }
                    Err(_) => broken = true,
                }
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
    let mut bits =
        std::mem::size_of::<usize>() as u32 * 8 - (enum_.values.len() - 1).leading_zeros();
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
            errors.push(Output::error_with_note(
                e.loc,
                format!("duplicate enum value {}", e.name),
                prev.0.clone(),
                "location of previous definition".to_string(),
            ));
            continue;
        }

        entries.insert(e.name.to_string(), (e.loc, i));
    }

    EnumDecl {
        name: enum_.name.name.to_string(),
        ty: ast::PrimitiveType::Uint(bits as u16),
        values: entries,
    }
}

#[test]
fn enum_256values_is_uint8() {
    let mut e = ast::EnumDefinition {
        doc: vec![],
        name: ast::Identifier {
            loc: ast::Loc(0, 0),
            name: "foo".into(),
        },
        values: Vec::new(),
    };

    e.values.push(ast::Identifier {
        loc: ast::Loc(0, 0),
        name: "first".into(),
    });

    let f = enum_decl(&e, &mut Vec::new());
    assert_eq!(f.ty, ast::PrimitiveType::Uint(8));

    for i in 1..256 {
        e.values.push(ast::Identifier {
            loc: ast::Loc(0, 0),
            name: format!("val{}", i),
        })
    }

    assert_eq!(e.values.len(), 256);

    let r = enum_decl(&e, &mut Vec::new());
    assert_eq!(r.ty, ast::PrimitiveType::Uint(8));

    e.values.push(ast::Identifier {
        loc: ast::Loc(0, 0),
        name: "another".into(),
    });

    let r2 = enum_decl(&e, &mut Vec::new());
    assert_eq!(r2.ty, ast::PrimitiveType::Uint(16));
}
