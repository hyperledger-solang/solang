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
extern crate tiny_keccak;
extern crate wasmi;

use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use std::collections::HashMap;
use tiny_keccak::keccak256;
use wasmi::memory_units::Pages;
use wasmi::*;

use solang::abi;
use solang::output;
use solang::{compile, Target};

mod substrate_enums;

#[allow(clippy::unreadable_literal, clippy::naive_bytecount)]
mod substrate_expressions;

mod substrate_arrays;
mod substrate_calls;
mod substrate_first;
mod substrate_functions;
mod substrate_mappings;
mod substrate_primitives;
mod substrate_strings;
mod substrate_structs;

type StorageKey = [u8; 32];

#[derive(FromPrimitive)]
#[allow(non_camel_case_types)]
enum SubstrateExternal {
    ext_scratch_size = 0,
    ext_scratch_read,
    ext_scratch_write,
    ext_set_storage,
    ext_clear_storage,
    ext_get_storage,
    ext_return,
    ext_hash_keccak_256,
    ext_print,
}

pub struct ContractStorage {
    memory: MemoryRef,
    pub scratch: Vec<u8>,
    pub store: HashMap<StorageKey, Vec<u8>>,
}

impl ContractStorage {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        ContractStorage {
            memory: MemoryInstance::alloc(Pages(16), Some(Pages(16))).unwrap(),
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
                    panic!("ext_scratch_read: {}", e);
                }

                Ok(None)
            }
            Some(SubstrateExternal::ext_scratch_write) => {
                let dest: u32 = args.nth_checked(0)?;
                let len: u32 = args.nth_checked(1)?;

                self.scratch.resize(len as usize, 0u8);

                if let Err(e) = self.memory.get_into(dest, &mut self.scratch) {
                    panic!("ext_scratch_write: {}", e);
                }

                Ok(None)
            }
            Some(SubstrateExternal::ext_get_storage) => {
                assert_eq!(args.len(), 1);

                let key_ptr: u32 = args.nth_checked(0)?;

                let mut key: StorageKey = [0; 32];

                if let Err(e) = self.memory.get_into(key_ptr, &mut key) {
                    panic!("ext_get_storage: {}", e);
                }

                if self.store.contains_key(&key) {
                    self.scratch = self.store[&key].clone();
                    println!("ext_get_storage: {:?} = {:?}", key, self.scratch);
                    Ok(Some(RuntimeValue::I32(0)))
                } else {
                    self.scratch.clear();
                    println!("ext_get_storage: {:?} = nil", key);
                    Ok(Some(RuntimeValue::I32(1)))
                }
            }
            Some(SubstrateExternal::ext_clear_storage) => {
                let key_ptr: u32 = args.nth_checked(0)?;

                let mut key: StorageKey = [0; 32];

                if let Err(e) = self.memory.get_into(key_ptr, &mut key) {
                    panic!("ext_clear_storage: {}", e);
                }

                println!("ext_clear_storage: {:?}", key);
                self.store.remove(&key);

                Ok(None)
            }
            Some(SubstrateExternal::ext_set_storage) => {
                assert_eq!(args.len(), 3);

                let key_ptr: u32 = args.nth_checked(0)?;
                let data_ptr: u32 = args.nth_checked(1)?;
                let len: u32 = args.nth_checked(2)?;

                let mut key: StorageKey = [0; 32];

                if let Err(e) = self.memory.get_into(key_ptr, &mut key) {
                    panic!("ext_set_storage: {}", e);
                }

                let mut data = Vec::new();
                data.resize(len as usize, 0u8);

                if let Err(e) = self.memory.get_into(data_ptr, &mut data) {
                    panic!("ext_set_storage: {}", e);
                }
                println!("ext_set_storage: {:?} = {:?}", key, data);

                self.store.insert(key, data);

                Ok(None)
            }
            Some(SubstrateExternal::ext_hash_keccak_256) => {
                let data_ptr: u32 = args.nth_checked(0)?;
                let len: u32 = args.nth_checked(1)?;
                let out_ptr: u32 = args.nth_checked(2)?;

                let mut data = Vec::new();

                data.resize(len as usize, 0);

                if let Err(e) = self.memory.get_into(data_ptr, &mut data) {
                    panic!("ext_hash_keccak_256: {}", e);
                }
                let hash = keccak256(&data);

                if let Err(e) = self.memory.set(out_ptr, &hash) {
                    panic!("ext_hash_keccak_256: {}", e);
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
            Some(SubstrateExternal::ext_print) => {
                let data_ptr: u32 = args.nth_checked(0)?;
                let len: u32 = args.nth_checked(1)?;

                let mut buf = Vec::new();
                buf.resize(len as usize, 0u8);

                if let Err(e) = self.memory.get_into(data_ptr, &mut buf) {
                    panic!("ext_print: {}", e);
                }

                println!("{}", String::from_utf8_lossy(&buf));

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
            "ext_clear_storage" => SubstrateExternal::ext_clear_storage,
            "ext_return" => SubstrateExternal::ext_return,
            "ext_hash_keccak_256" => SubstrateExternal::ext_hash_keccak_256,
            "ext_print" => SubstrateExternal::ext_print,
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
    pub fn constructor(&self, store: &mut ContractStorage, index: usize, args: Vec<u8>) {
        let m = &self.abi.contract.constructors[index];

        store.scratch = m.selector().into_iter().chain(args).collect();

        if let Some(RuntimeValue::I32(ret)) = self
            .module
            .invoke_export("deploy", &[], store)
            .expect("failed to call function")
        {
            if ret != 0 {
                panic!("non zero return")
            }
        }
    }

    pub fn function(&self, store: &mut ContractStorage, name: &str, args: Vec<u8>) {
        let m = self.abi.get_function(name).unwrap();

        store.scratch = m.selector().into_iter().chain(args).collect();

        if let Some(RuntimeValue::I32(ret)) = self
            .module
            .invoke_export("call", &[], store)
            .expect("failed to call function")
        {
            if ret != 0 {
                panic!("non zero return")
            }
        }
    }

    pub fn function_expect_revert(&self, store: &mut ContractStorage, name: &str, args: Vec<u8>) {
        let m = self.abi.get_function(name).unwrap();

        store.scratch = m.selector().into_iter().chain(args).collect();

        if let Some(RuntimeValue::I32(ret)) = self
            .module
            .invoke_export("call", &[], store)
            .expect("failed to call function")
        {
            if ret != 1 {
                panic!("non one return")
            }
        }
    }

    pub fn raw_function(&self, store: &mut ContractStorage, input: Vec<u8>) {
        store.scratch = input;

        if let Some(RuntimeValue::I32(ret)) = self
            .module
            .invoke_export("call", &[], store)
            .expect("failed to call function")
        {
            if ret != 0 {
                panic!("non zero return")
            }
        }
    }

    pub fn raw_constructor(&self, store: &mut ContractStorage, input: Vec<u8>) {
        store.scratch = input;

        if let Some(RuntimeValue::I32(ret)) = self
            .module
            .invoke_export("deploy", &[], store)
            .expect("failed to call constructor")
        {
            if ret != 0 {
                panic!("non zero return")
            }
        }
    }
}

pub fn build_solidity(src: &'static str) -> (TestRuntime, ContractStorage) {
    let (mut res, errors) = compile(
        src,
        "test.sol",
        inkwell::OptimizationLevel::Default,
        Target::Substrate,
    );

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
    match errors.iter().find(|m| m.level == output::Level::Error) {
        Some(m) => m.message.to_owned(),
        None => panic!("no errors found"),
    }
}

pub fn first_warning(errors: Vec<output::Output>) -> String {
    match errors.iter().find(|m| m.level == output::Level::Warning) {
        Some(m) => m.message.to_owned(),
        None => panic!("no warnings found"),
    }
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
