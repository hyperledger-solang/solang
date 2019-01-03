
use ast::ElementaryTypeName;
use std::collections::HashMap;
use llvm_sys::prelude::LLVMValueRef;

#[derive(Debug)]
pub struct Variable {
    pub typ: ElementaryTypeName,
    pub value: LLVMValueRef,
}

pub struct Vartable {
    vars: Vec<HashMap<String, Variable>>
}

impl Vartable {
    pub fn new() -> Self {
        Vartable{vars: vec!(HashMap::new())}
    }

    pub fn insert(&mut self, name: &String, typ: ElementaryTypeName, value: LLVMValueRef) {
        self.vars[0].insert(name.to_string(), Variable{typ, value});
    }

    pub fn get_type(&self, name: &String) -> ElementaryTypeName {
        self.get(name).typ
    }

    pub fn get_value(&self, name: &String) -> LLVMValueRef {
        self.get(name).value
    }

    pub fn set_value(&mut self, name: &String, value: LLVMValueRef) {
        if self.vars[0].contains_key(name) {
            self.vars[0].get_mut(name).unwrap().value = value;
        } else {
            let typ = self.get_type(name);
            self.vars[0].insert(name.to_string(), Variable{typ, value});
        }
    }

    pub fn get(&self, name: &String) -> &Variable {
        for scope in &self.vars {
            if scope.contains_key(name) {
                return scope.get(name).unwrap()
            }
        }
        panic!("variable {} not found", name);
    }

    pub fn new_scope(&mut self) {
        self.vars.insert(0, HashMap::new())
    }

    pub fn leave_scope(&mut self) -> HashMap<String, Variable> {
        self.vars.remove(0)
    }
}