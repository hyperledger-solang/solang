// Create WASM virtual machine like substrate
extern crate ethabi;
extern crate ethereum_types;
extern crate num_bigint;
extern crate num_derive;
extern crate num_traits;
extern crate parity_scale_codec;
extern crate parity_scale_codec_derive;
extern crate rand;
extern crate serde_derive;
extern crate solang;
extern crate wasmi;

use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use std::collections::HashMap;
use wasmi::memory_units::Pages;
use wasmi::*;

use solang::abi;
use solang::output;
use solang::{compile, Target};

mod substrate_enums;
mod substrate_expressions;
mod substrate_first;
mod substrate_functions;
mod substrate_primitives;

type StorageKey = [u8; 32];

#[derive(FromPrimitive)]
#[allow(non_camel_case_types)]
enum SubstrateExternal {
    ext_scratch_size = 0,
    ext_scratch_read,
    ext_scratch_write,
    ext_set_storage,
    ext_get_storage,
    ext_return,
}

pub struct ContractStorage {
    memory: MemoryRef,
    pub scratch: Vec<u8>,
    pub store: HashMap<StorageKey, Vec<u8>>,
}

impl ContractStorage {
    pub fn new() -> Self {
        ContractStorage {
            memory: MemoryInstance::alloc(Pages(2), Some(Pages(2))).unwrap(),
            scratch: Vec::new(),
            store: HashMap::new(),
        }
    }
}

impl Externals for ContractStorage {
    fn invoke_index(
        &mut self,
        index: usize,
        args: RuntimeArgs,
    ) -> Result<Option<RuntimeValue>, Trap> {
        match FromPrimitive::from_usize(index) {
            Some(SubstrateExternal::ext_scratch_size) => {
                Ok(Some(RuntimeValue::I32(self.scratch.len() as i32)))
            }
            Some(SubstrateExternal::ext_scratch_read) => {
                let dest: u32 = args.nth_checked(0)?;
                let offset: u32 = args.nth_checked(1)?;
                let len: u32 = args.nth_checked(2)?;

                if let Err(e) = self.memory.set(
                    dest,
                    &self.scratch[offset as usize..(offset + len) as usize],
                ) {
                    panic!("ext_set_storage: {}", e);
                }

                Ok(None)
            }
            Some(SubstrateExternal::ext_scratch_write) => {
                let dest: u32 = args.nth_checked(0)?;
                let len: u32 = args.nth_checked(1)?;

                self.scratch.resize(len as usize, 0u8);

                if let Err(e) = self.memory.get_into(dest, &mut self.scratch) {
                    panic!("ext_set_storage: {}", e);
                }

                Ok(None)
            }
            Some(SubstrateExternal::ext_get_storage) => {
                assert_eq!(args.len(), 1);

                let key_ptr: u32 = args.nth_checked(0)?;

                let mut key: StorageKey = [0; 32];

                if let Err(e) = self.memory.get_into(key_ptr, &mut key) {
                    panic!("ext_set_storage: {}", e);
                }

                if self.store.contains_key(&key) {
                    self.scratch = self.store[&key].clone();
                    Ok(Some(RuntimeValue::I32(0)))
                } else {
                    self.scratch.clear();
                    Ok(Some(RuntimeValue::I32(1)))
                }
            }
            Some(SubstrateExternal::ext_set_storage) => {
                assert_eq!(args.len(), 4);

                let key_ptr: u32 = args.nth_checked(0)?;
                let value_non_null: u32 = args.nth_checked(1)?;
                let data_ptr: u32 = args.nth_checked(2)?;
                let len: u32 = args.nth_checked(3)?;

                let mut key: StorageKey = [0; 32];

                if let Err(e) = self.memory.get_into(key_ptr, &mut key) {
                    panic!("ext_set_storage: {}", e);
                }

                if value_non_null != 0 {
                    let mut data = Vec::new();
                    data.resize(len as usize, 0u8);

                    if let Err(e) = self.memory.get_into(data_ptr, &mut data) {
                        panic!("ext_set_storage: {}", e);
                    }

                    self.store.insert(key, data);
                } else {
                    self.store.remove(&key);
                }

                Ok(None)
            }
            Some(SubstrateExternal::ext_return) => {
                let data_ptr: u32 = args.nth_checked(0)?;
                let len: u32 = args.nth_checked(1)?;

                self.scratch.resize(len as usize, 0u8);

                if let Err(e) = self.memory.get_into(data_ptr, &mut self.scratch) {
                    panic!("ext_return: {}", e);
                }

                Ok(None)
            }
            _ => panic!("external {} unknown", index),
        }
    }
}

impl ModuleImportResolver for ContractStorage {
    fn resolve_func(&self, field_name: &str, signature: &Signature) -> Result<FuncRef, Error> {
        let index = match field_name {
            "ext_scratch_size" => SubstrateExternal::ext_scratch_size,
            "ext_scratch_read" => SubstrateExternal::ext_scratch_read,
            "ext_scratch_write" => SubstrateExternal::ext_scratch_write,
            "ext_get_storage" => SubstrateExternal::ext_get_storage,
            "ext_set_storage" => SubstrateExternal::ext_set_storage,
            "ext_return" => SubstrateExternal::ext_return,
            _ => {
                panic!("{} not implemented", field_name);
            }
        };

        Ok(FuncInstance::alloc_host(signature.clone(), index as usize))
    }

    fn resolve_memory(
        &self,
        _field_name: &str,
        _memory_type: &MemoryDescriptor,
    ) -> Result<MemoryRef, Error> {
        Ok(self.memory.clone())
    }
}

pub struct TestRuntime {
    module: ModuleRef,
    abi: abi::substrate::Metadata,
}

impl TestRuntime {
    pub fn constructor<'a>(&self, store: &mut ContractStorage, index: usize, args: Vec<u8>) {
        let m = &self.abi.contract.constructors[index];

        store.scratch = m
            .selector
            .to_le_bytes()
            .to_vec()
            .into_iter()
            .chain(args)
            .collect();

        self.module
            .invoke_export("deploy", &[], store)
            .expect("failed to call function");
    }

    pub fn function<'a>(&self, store: &mut ContractStorage, name: &str, args: Vec<u8>) {
        let m = self.abi.get_function(name).unwrap();

        store.scratch = m
            .selector
            .to_le_bytes()
            .to_vec()
            .into_iter()
            .chain(args)
            .collect();

        self.module
            .invoke_export("call", &[], store)
            .expect("failed to call function");
    }

    pub fn raw_function<'a>(&self, store: &mut ContractStorage, input: Vec<u8>) {
        store.scratch = input;

        self.module
            .invoke_export("call", &[], store)
            .expect("failed to call function");
    }
}

pub fn build_solidity(src: &'static str) -> (TestRuntime, ContractStorage) {
    let (mut res, errors) = compile(src, "test.sol", "default", &Target::Substrate);

    output::print_messages("test.sol", src, &errors, false);

    assert_eq!(res.len(), 1);

    // resolve
    let (bc, abistr) = res.pop().unwrap();

    let module = Module::from_buffer(bc).expect("parse wasm should work");

    let store = ContractStorage::new();

    (
        TestRuntime {
            module: ModuleInstance::new(
                &module,
                &ImportsBuilder::new().with_resolver("env", &store),
            )
            .expect("Failed to instantiate module")
            .run_start(&mut NopExternals)
            .expect("Failed to run start function in module"),
            abi: abi::substrate::load(&abistr).unwrap(),
        },
        store,
    )
}

pub fn first_error(errors: Vec<output::Output>) -> String {
    for m in errors.iter().filter(|m| m.level == output::Level::Error) {
        return m.message.to_owned();
    }

    panic!("no errors detected");
}

pub fn first_warning(errors: Vec<output::Output>) -> String {
    for m in errors.iter().filter(|m| m.level == output::Level::Warning) {
        return m.message.to_owned();
    }

    panic!("no errors detected");
}

pub fn no_errors(errors: Vec<output::Output>) {
    assert!(
        errors
            .iter()
            .filter(|m| m.level == output::Level::Error)
            .count()
            == 0
    );
}
