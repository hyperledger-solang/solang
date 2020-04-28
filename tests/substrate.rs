// Create WASM virtual machine like substrate
extern crate blake2_rfc;
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
use rand::Rng;
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
mod substrate_contracts;
mod substrate_first;
mod substrate_functions;
mod substrate_mappings;
mod substrate_primitives;
mod substrate_strings;
mod substrate_structs;

type StorageKey = [u8; 32];
type Address = [u8; 32];

fn address_new() -> Address {
    let mut rng = rand::thread_rng();

    let mut a = [0u8; 32];

    rng.fill(&mut a[..]);

    a
}

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
    ext_call,
    ext_instantiate,
}

pub struct VM {
    address: Address,
    memory: MemoryRef,
    pub scratch: Vec<u8>,
}

impl VM {
    fn new(address: Address) -> Self {
        VM {
            memory: MemoryInstance::alloc(Pages(16), Some(Pages(16))).unwrap(),
            scratch: Vec::new(),
            address,
        }
    }
}

pub struct TestRuntime {
    pub store: HashMap<(Address, StorageKey), Vec<u8>>,
    pub contracts: Vec<(Vec<u8>, String)>,
    pub printbuf: String,
    pub accounts: HashMap<Address, Vec<u8>>,
    pub abi: abi::substrate::Metadata,
    pub vm: VM,
}

impl Externals for TestRuntime {
    fn invoke_index(
        &mut self,
        index: usize,
        args: RuntimeArgs,
    ) -> Result<Option<RuntimeValue>, Trap> {
        match FromPrimitive::from_usize(index) {
            Some(SubstrateExternal::ext_scratch_size) => {
                Ok(Some(RuntimeValue::I32(self.vm.scratch.len() as i32)))
            }
            Some(SubstrateExternal::ext_scratch_read) => {
                let dest: u32 = args.nth_checked(0)?;
                let offset: u32 = args.nth_checked(1)?;
                let len: u32 = args.nth_checked(2)?;

                println!(
                    "ext_scratch_read({}, {}, {}) scratch={} {}",
                    dest,
                    offset,
                    len,
                    self.vm.scratch.len(),
                    hex::encode(&self.vm.scratch)
                );

                if let Err(e) = self.vm.memory.set(
                    dest,
                    &self.vm.scratch[offset as usize..(offset + len) as usize],
                ) {
                    panic!("ext_scratch_read: {}", e);
                }

                Ok(None)
            }
            Some(SubstrateExternal::ext_scratch_write) => {
                let dest: u32 = args.nth_checked(0)?;
                let len: u32 = args.nth_checked(1)?;

                self.vm.scratch.resize(len as usize, 0u8);

                if let Err(e) = self.vm.memory.get_into(dest, &mut self.vm.scratch) {
                    panic!("ext_scratch_write: {}", e);
                }

                Ok(None)
            }
            Some(SubstrateExternal::ext_get_storage) => {
                assert_eq!(args.len(), 1);

                let key_ptr: u32 = args.nth_checked(0)?;

                let mut key: StorageKey = [0; 32];

                if let Err(e) = self.vm.memory.get_into(key_ptr, &mut key) {
                    panic!("ext_get_storage: {}", e);
                }

                if let Some(value) = self.store.get(&(self.vm.address, key)) {
                    self.vm.scratch = value.clone();
                    println!("ext_get_storage: {:?} = {:?}", key, self.vm.scratch);
                    Ok(Some(RuntimeValue::I32(0)))
                } else {
                    self.vm.scratch.clear();
                    println!("ext_get_storage: {:?} = nil", key);
                    Ok(Some(RuntimeValue::I32(1)))
                }
            }
            Some(SubstrateExternal::ext_clear_storage) => {
                let key_ptr: u32 = args.nth_checked(0)?;

                let mut key: StorageKey = [0; 32];

                if let Err(e) = self.vm.memory.get_into(key_ptr, &mut key) {
                    panic!("ext_clear_storage: {}", e);
                }

                println!("ext_clear_storage: {:?}", key);
                self.store.remove(&(self.vm.address, key));

                Ok(None)
            }
            Some(SubstrateExternal::ext_set_storage) => {
                assert_eq!(args.len(), 3);

                let key_ptr: u32 = args.nth_checked(0)?;
                let data_ptr: u32 = args.nth_checked(1)?;
                let len: u32 = args.nth_checked(2)?;

                let mut key: StorageKey = [0; 32];

                if let Err(e) = self.vm.memory.get_into(key_ptr, &mut key) {
                    panic!("ext_set_storage: {}", e);
                }

                let mut data = Vec::new();
                data.resize(len as usize, 0u8);

                if let Err(e) = self.vm.memory.get_into(data_ptr, &mut data) {
                    panic!("ext_set_storage: {}", e);
                }
                println!("ext_set_storage: {:?} = {:?}", key, data);

                self.store.insert((self.vm.address, key), data);

                Ok(None)
            }
            Some(SubstrateExternal::ext_hash_keccak_256) => {
                let data_ptr: u32 = args.nth_checked(0)?;
                let len: u32 = args.nth_checked(1)?;
                let out_ptr: u32 = args.nth_checked(2)?;

                let mut data = Vec::new();

                data.resize(len as usize, 0);

                if let Err(e) = self.vm.memory.get_into(data_ptr, &mut data) {
                    panic!("ext_hash_keccak_256: {}", e);
                }
                let hash = keccak256(&data);

                if let Err(e) = self.vm.memory.set(out_ptr, &hash) {
                    panic!("ext_hash_keccak_256: {}", e);
                }

                Ok(None)
            }
            Some(SubstrateExternal::ext_return) => {
                let data_ptr: u32 = args.nth_checked(0)?;
                let len: u32 = args.nth_checked(1)?;

                self.vm.scratch.resize(len as usize, 0u8);

                if let Err(e) = self.vm.memory.get_into(data_ptr, &mut self.vm.scratch) {
                    panic!("ext_return: {}", e);
                }

                Ok(None)
            }
            Some(SubstrateExternal::ext_print) => {
                let data_ptr: u32 = args.nth_checked(0)?;
                let len: u32 = args.nth_checked(1)?;

                let mut buf = Vec::new();
                buf.resize(len as usize, 0u8);

                if let Err(e) = self.vm.memory.get_into(data_ptr, &mut buf) {
                    panic!("ext_print: {}", e);
                }

                let s = String::from_utf8_lossy(&buf);

                println!("{}", s);

                self.printbuf.push_str(&s);

                Ok(None)
            }
            Some(SubstrateExternal::ext_call) => {
                let address_ptr: u32 = args.nth_checked(0)?;
                let address_len: u32 = args.nth_checked(1)?;
                let input_ptr: u32 = args.nth_checked(5)?;
                let input_len: u32 = args.nth_checked(6)?;

                let mut address = [0u8; 32];

                if address_len != 32 {
                    panic!("ext_call: len = {}", address_len);
                }

                if let Err(e) = self.vm.memory.get_into(address_ptr, &mut address) {
                    panic!("ext_call: {}", e);
                }

                let mut input = Vec::new();
                input.resize(input_len as usize, 0u8);

                if let Err(e) = self.vm.memory.get_into(input_ptr, &mut input) {
                    panic!("ext_call: {}", e);
                }

                let mut vm = VM::new(address);

                std::mem::swap(&mut self.vm, &mut vm);

                let module = self.create_module(self.accounts.get(&self.vm.address).unwrap());

                self.vm.scratch = input;
                if let Some(RuntimeValue::I32(ret)) = module
                    .invoke_export("call", &[], self)
                    .expect("failed to call function")
                {
                    if ret != 0 {
                        panic!("non zero return")
                    }
                }

                let output = self.vm.scratch.clone();

                std::mem::swap(&mut self.vm, &mut vm);

                self.vm.scratch = output;

                Ok(Some(RuntimeValue::I32(0)))
            }
            Some(SubstrateExternal::ext_instantiate) => {
                let codehash_ptr: u32 = args.nth_checked(0)?;
                let codehash_len: u32 = args.nth_checked(1)?;
                let input_ptr: u32 = args.nth_checked(5)?;
                let input_len: u32 = args.nth_checked(6)?;

                let mut codehash = [0u8; 32];

                if codehash_len != 32 {
                    panic!("ext_instantiate: len = {}", codehash_len);
                }

                if let Err(e) = self.vm.memory.get_into(codehash_ptr, &mut codehash) {
                    panic!("ext_instantiate: {}", e);
                }

                let address = address_new();

                let code = self
                    .contracts
                    .iter()
                    .find(|code| {
                        blake2_rfc::blake2b::blake2b(32, &[], &code.0).as_bytes() == codehash
                    })
                    .expect("codehash not found");

                self.accounts.insert(address, code.0.clone());

                let mut input = Vec::new();
                input.resize(input_len as usize, 0u8);

                if let Err(e) = self.vm.memory.get_into(input_ptr, &mut input) {
                    panic!("ext_instantiate: {}", e);
                }

                let mut vm = VM::new(address);

                std::mem::swap(&mut self.vm, &mut vm);

                let module = self.create_module(&code.0);

                self.vm.scratch = input;
                if let Some(RuntimeValue::I32(ret)) = module
                    .invoke_export("deploy", &[], self)
                    .expect("failed to call constructor")
                {
                    if ret != 0 {
                        panic!("non zero return")
                    }
                }

                std::mem::swap(&mut self.vm, &mut vm);

                self.vm.scratch = address.to_vec();

                Ok(Some(RuntimeValue::I32(0)))
            }
            _ => panic!("external {} unknown", index),
        }
    }
}

impl ModuleImportResolver for TestRuntime {
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
            "ext_call" => SubstrateExternal::ext_call,
            "ext_instantiate" => SubstrateExternal::ext_instantiate,
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
        Ok(self.vm.memory.clone())
    }
}

impl TestRuntime {
    fn create_module(&self, code: &[u8]) -> ModuleRef {
        let module = Module::from_buffer(&code).expect("parse wasm should work");

        ModuleInstance::new(&module, &ImportsBuilder::new().with_resolver("env", self))
            .expect("Failed to instantiate module")
            .run_start(&mut NopExternals)
            .expect("Failed to run start function in module")
    }

    pub fn constructor(&mut self, index: usize, args: Vec<u8>) {
        let m = &self.abi.contract.constructors[index];

        let module = self.create_module(self.accounts.get(&self.vm.address).unwrap());

        self.vm.scratch = m.selector().into_iter().chain(args).collect();

        if let Some(RuntimeValue::I32(ret)) = module
            .invoke_export("deploy", &[], self)
            .expect("failed to call function")
        {
            if ret != 0 {
                panic!("non zero return")
            }
        }
    }

    pub fn function(&mut self, name: &str, args: Vec<u8>) {
        let m = self.abi.get_function(name).unwrap();

        let module = self.create_module(self.accounts.get(&self.vm.address).unwrap());

        self.vm.scratch = m.selector().into_iter().chain(args).collect();

        if let Some(RuntimeValue::I32(ret)) = module
            .invoke_export("call", &[], self)
            .expect("failed to call function")
        {
            if ret != 0 {
                panic!("non zero return")
            }
        }
    }

    pub fn function_expect_return(&mut self, name: &str, args: Vec<u8>, expected_ret: i32) {
        let m = self.abi.get_function(name).unwrap();

        let module = self.create_module(self.accounts.get(&self.vm.address).unwrap());

        self.vm.scratch = m.selector().into_iter().chain(args).collect();

        if let Some(RuntimeValue::I32(ret)) = module
            .invoke_export("call", &[], self)
            .expect("failed to call function")
        {
            if expected_ret != ret {
                panic!("non one return")
            }
        }
    }

    pub fn raw_function(&mut self, input: Vec<u8>) {
        let module = self.create_module(self.accounts.get(&self.vm.address).unwrap());

        self.vm.scratch = input;

        if let Some(RuntimeValue::I32(ret)) = module
            .invoke_export("call", &[], self)
            .expect("failed to call function")
        {
            if ret != 0 {
                panic!("non zero return")
            }
        }
    }

    pub fn raw_constructor(&mut self, input: Vec<u8>) {
        let module = self.create_module(self.accounts.get(&self.vm.address).unwrap());

        self.vm.scratch = input;

        if let Some(RuntimeValue::I32(ret)) = module
            .invoke_export("deploy", &[], self)
            .expect("failed to call constructor")
        {
            if ret != 0 {
                panic!("non zero return")
            }
        }
    }
}

pub fn build_solidity(src: &'static str) -> TestRuntime {
    let (res, errors) = compile(
        src,
        "test.sol",
        inkwell::OptimizationLevel::Default,
        Target::Substrate,
    );

    output::print_messages("test.sol", src, &errors, false);

    assert!(!res.is_empty());

    let abistr = res[0].1.clone();
    let code = res[0].0.clone();
    let address = address_new();

    let mut t = TestRuntime {
        accounts: HashMap::new(),
        printbuf: String::new(),
        store: HashMap::new(),
        contracts: res,
        vm: VM::new(address),
        abi: abi::substrate::load(&abistr).unwrap(),
    };

    t.accounts.insert(address, code);

    t
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
