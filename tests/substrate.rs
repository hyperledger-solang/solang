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
extern crate sha2;
extern crate solang;
extern crate tiny_keccak;
extern crate wasmi;

use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use rand::Rng;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fmt;
use tiny_keccak::{Hasher, Keccak};
use wasmi::memory_units::Pages;
use wasmi::*;

use solang::abi;
use solang::file_cache::FileCache;
use solang::sema::ast;
use solang::sema::diagnostics;
use solang::{compile, Target};

mod substrate_enums;

#[allow(clippy::unreadable_literal, clippy::naive_bytecount)]
mod substrate_expressions;

mod substrate_arrays;
mod substrate_builtins;
mod substrate_calls;
mod substrate_contracts;
mod substrate_first;
mod substrate_functions;
mod substrate_imports;
mod substrate_inheritance;
mod substrate_loops;
mod substrate_mappings;
mod substrate_primitives;
mod substrate_strings;
mod substrate_structs;
mod substrate_value;
mod substrate_variables;

type StorageKey = [u8; 32];
type Address = [u8; 32];

fn address_new() -> Address {
    let mut rng = rand::thread_rng();

    let mut a = [0u8; 32];

    rng.fill(&mut a[..]);

    a
}

#[derive(Debug, Clone, PartialEq)]
struct HostCodeTerminate {}

impl HostError for HostCodeTerminate {}

impl fmt::Display for HostCodeTerminate {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "ext_terminate")
    }
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
    ext_println,
    ext_call,
    ext_instantiate,
    ext_value_transferred,
    ext_minimum_balance,
    ext_random,
    ext_address,
    ext_balance,
    ext_terminate,
    ext_hash_sha2_256,
    ext_hash_blake2_128,
    ext_hash_blake2_256,
    ext_block_number,
    ext_now,
    ext_gas_price,
    ext_gas_left,
    ext_caller,
    ext_tombstone_deposit,
}

pub struct VM {
    address: Address,
    caller: Address,
    memory: MemoryRef,
    pub scratch: Vec<u8>,
    pub value: u128,
}

impl VM {
    fn new(address: Address, caller: Address, value: u128) -> Self {
        VM {
            memory: MemoryInstance::alloc(Pages(16), Some(Pages(16))).unwrap(),
            scratch: Vec::new(),
            address,
            caller,
            value,
        }
    }
}

pub struct TestRuntime {
    pub store: HashMap<(Address, StorageKey), Vec<u8>>,
    pub contracts: Vec<(Vec<u8>, String)>,
    pub printbuf: String,
    pub accounts: HashMap<Address, (Vec<u8>, u128)>,
    pub abi: abi::substrate::Metadata,
    pub vm: VM,
}

impl Externals for TestRuntime {
    #[allow(clippy::cognitive_complexity)]
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

                println!(
                    "ext_scratch_write({}, {})",
                    len,
                    hex::encode(&self.vm.scratch)
                );

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

                let mut hasher = Keccak::v256();
                let mut hash = [0u8; 32];
                hasher.update(&data);
                hasher.finalize(&mut hash);

                println!(
                    "ext_hash_keccak_256: {} = {}",
                    hex::encode(data),
                    hex::encode(hash)
                );

                if let Err(e) = self.vm.memory.set(out_ptr, &hash) {
                    panic!("ext_hash_keccak_256: {}", e);
                }

                Ok(None)
            }
            Some(SubstrateExternal::ext_hash_sha2_256) => {
                let data_ptr: u32 = args.nth_checked(0)?;
                let len: u32 = args.nth_checked(1)?;
                let out_ptr: u32 = args.nth_checked(2)?;

                let mut data = Vec::new();

                data.resize(len as usize, 0);

                if let Err(e) = self.vm.memory.get_into(data_ptr, &mut data) {
                    panic!("ext_hash_sha2_256: {}", e);
                }

                let mut hasher = Sha256::new();

                hasher.input(&data);

                let hash = hasher.result();

                println!(
                    "ext_hash_sha2_256: {} = {}",
                    hex::encode(data),
                    hex::encode(hash)
                );

                if let Err(e) = self.vm.memory.set(out_ptr, &hash) {
                    panic!("ext_hash_sha2_256: {}", e);
                }

                Ok(None)
            }
            Some(SubstrateExternal::ext_hash_blake2_128) => {
                let data_ptr: u32 = args.nth_checked(0)?;
                let len: u32 = args.nth_checked(1)?;
                let out_ptr: u32 = args.nth_checked(2)?;

                let mut data = Vec::new();

                data.resize(len as usize, 0);

                if let Err(e) = self.vm.memory.get_into(data_ptr, &mut data) {
                    panic!("ext_hash_blake2_128: {}", e);
                }
                let hash = blake2_rfc::blake2b::blake2b(16, &[], &data);

                println!(
                    "ext_hash_blake2_128: {} = {}",
                    hex::encode(data),
                    hex::encode(hash)
                );

                if let Err(e) = self.vm.memory.set(out_ptr, &hash.as_bytes()) {
                    panic!("ext_hash_blake2_128: {}", e);
                }

                Ok(None)
            }
            Some(SubstrateExternal::ext_hash_blake2_256) => {
                let data_ptr: u32 = args.nth_checked(0)?;
                let len: u32 = args.nth_checked(1)?;
                let out_ptr: u32 = args.nth_checked(2)?;

                let mut data = Vec::new();

                data.resize(len as usize, 0);

                if let Err(e) = self.vm.memory.get_into(data_ptr, &mut data) {
                    panic!("ext_hash_blake2_256: {}", e);
                }

                let hash = blake2_rfc::blake2b::blake2b(32, &[], &data);

                println!(
                    "ext_hash_blake2_256: {} = {}",
                    hex::encode(data),
                    hex::encode(hash)
                );

                if let Err(e) = self.vm.memory.set(out_ptr, &hash.as_bytes()) {
                    panic!("ext_hash_blake2_256: {}", e);
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
            Some(SubstrateExternal::ext_println) => {
                let data_ptr: u32 = args.nth_checked(0)?;
                let len: u32 = args.nth_checked(1)?;

                let mut buf = Vec::new();
                buf.resize(len as usize, 0u8);

                if let Err(e) = self.vm.memory.get_into(data_ptr, &mut buf) {
                    panic!("ext_println: {}", e);
                }

                let s = String::from_utf8_lossy(&buf);

                println!("ext_println: {}", s);

                self.printbuf.push_str(&s);

                Ok(None)
            }
            Some(SubstrateExternal::ext_random) => {
                let data_ptr: u32 = args.nth_checked(0)?;
                let len: u32 = args.nth_checked(1)?;

                let mut buf = Vec::new();
                buf.resize(len as usize, 0u8);

                if let Err(e) = self.vm.memory.get_into(data_ptr, &mut buf) {
                    panic!("ext_random: {}", e);
                }

                let mut hash = [0u8; 32];

                hash.copy_from_slice(blake2_rfc::blake2b::blake2b(32, &[], &buf).as_bytes());

                println!("ext_random: {}", hex::encode(&hash));

                self.vm.scratch = hash.to_vec();

                Ok(None)
            }
            Some(SubstrateExternal::ext_call) => {
                let address_ptr: u32 = args.nth_checked(0)?;
                let address_len: u32 = args.nth_checked(1)?;
                //let gas: u64 = args.nth_checked(2)?;
                let value_ptr: u32 = args.nth_checked(3)?;
                let value_len: u32 = args.nth_checked(4)?;
                let input_ptr: u32 = args.nth_checked(5)?;
                let input_len: u32 = args.nth_checked(6)?;

                let mut address = [0u8; 32];

                if address_len != 32 {
                    panic!("ext_call: len = {}", address_len);
                }

                if let Err(e) = self.vm.memory.get_into(address_ptr, &mut address) {
                    panic!("ext_call: {}", e);
                }

                let mut value = [0u8; 16];

                if value_len != 16 {
                    panic!("ext_call: len = {}", value_len);
                }

                if let Err(e) = self.vm.memory.get_into(value_ptr, &mut value) {
                    panic!("ext_call: {}", e);
                }

                let value = u128::from_le_bytes(value);

                if !self.accounts.contains_key(&address) {
                    // substrate would return TRAP_RETURN_CODE (0x0100)
                    return Ok(Some(RuntimeValue::I32(0x100)));
                }

                let mut input = Vec::new();
                input.resize(input_len as usize, 0u8);

                if let Err(e) = self.vm.memory.get_into(input_ptr, &mut input) {
                    panic!("ext_call: {}", e);
                }

                println!(
                    "ext_call: address={} input={}",
                    hex::encode(address),
                    hex::encode(&input)
                );

                let mut vm = VM::new(address, self.vm.address, value);

                std::mem::swap(&mut self.vm, &mut vm);

                let module = self.create_module(&self.accounts.get(&self.vm.address).unwrap().0);

                self.vm.scratch = input;

                let ret = match module.invoke_export("call", &[], self) {
                    Err(wasmi::Error::Trap(trap)) => match trap.kind() {
                        TrapKind::Host(host_error) => {
                            if host_error.downcast_ref::<HostCodeTerminate>().is_some() {
                                Some(RuntimeValue::I32(1))
                            } else {
                                panic!("did not go as planned");
                            }
                        }
                        _ => panic!("fail to invoke main via create: {}", trap),
                    },
                    Ok(v) => v,
                    Err(e) => panic!("fail to invoke main via create: {}", e),
                };

                let output = self.vm.scratch.clone();

                std::mem::swap(&mut self.vm, &mut vm);

                println!("ext_call ret={:?} buf={}", ret, hex::encode(&output));

                if let Some(acc) = self.accounts.get_mut(&vm.address) {
                    acc.1 += vm.value;
                }
                self.vm.scratch = output;

                Ok(ret)
            }
            Some(SubstrateExternal::ext_instantiate) => {
                let codehash_ptr: u32 = args.nth_checked(0)?;
                let codehash_len: u32 = args.nth_checked(1)?;
                //let gas: u64 = args.nth_checked(2)?;
                let value_ptr: u32 = args.nth_checked(3)?;
                let value_len: u32 = args.nth_checked(4)?;
                let input_ptr: u32 = args.nth_checked(5)?;
                let input_len: u32 = args.nth_checked(6)?;

                let mut codehash = [0u8; 32];

                if codehash_len != 32 {
                    panic!("ext_instantiate: len = {}", codehash_len);
                }

                if let Err(e) = self.vm.memory.get_into(codehash_ptr, &mut codehash) {
                    panic!("ext_instantiate: {}", e);
                }

                let mut value = [0u8; 16];

                if value_len != 16 {
                    panic!("ext_instantiate: len = {}", value_len);
                }

                if let Err(e) = self.vm.memory.get_into(value_ptr, &mut value) {
                    panic!("ext_instantiate: {}", e);
                }

                let value = u128::from_le_bytes(value);

                let mut input = Vec::new();
                input.resize(input_len as usize, 0u8);

                if let Err(e) = self.vm.memory.get_into(input_ptr, &mut input) {
                    panic!("ext_instantiate: {}", e);
                }

                println!(
                    "ext_instantiate value:{} input={}",
                    value,
                    hex::encode(&input)
                );

                let mut address = [0u8; 32];

                address.copy_from_slice(blake2_rfc::blake2b::blake2b(32, &[], &input).as_bytes());

                if self.accounts.contains_key(&address) {
                    // substrate would return TRAP_RETURN_CODE (0x0100)
                    return Ok(Some(RuntimeValue::I32(0x100)));
                }

                let code = self
                    .contracts
                    .iter()
                    .find(|code| {
                        blake2_rfc::blake2b::blake2b(32, &[], &code.0).as_bytes() == codehash
                    })
                    .expect("codehash not found");

                self.accounts.insert(address, (code.0.clone(), 0));

                let mut input = Vec::new();
                input.resize(input_len as usize, 0u8);

                if let Err(e) = self.vm.memory.get_into(input_ptr, &mut input) {
                    panic!("ext_instantiate: {}", e);
                }

                let mut vm = VM::new(address, self.vm.address, value);

                std::mem::swap(&mut self.vm, &mut vm);

                let module = self.create_module(&code.0);

                self.vm.scratch = input;
                let ret = module
                    .invoke_export("deploy", &[], self)
                    .expect("failed to call constructor");

                let output = self.vm.scratch.clone();

                std::mem::swap(&mut self.vm, &mut vm);

                if let Some(RuntimeValue::I32(0)) = ret {
                    self.accounts.get_mut(&vm.address).unwrap().1 += vm.value;
                    self.vm.scratch = address.to_vec();
                } else {
                    self.vm.scratch = output;
                }

                Ok(ret)
            }
            Some(SubstrateExternal::ext_value_transferred) => {
                self.vm.scratch = self.vm.value.to_le_bytes().to_vec();

                println!("ext_value_transferred: {}", hex::encode(&self.vm.scratch));

                Ok(None)
            }
            Some(SubstrateExternal::ext_address) => {
                self.vm.scratch = self.vm.address.to_vec();

                println!("ext_address: {}", hex::encode(&self.vm.scratch));

                Ok(None)
            }
            Some(SubstrateExternal::ext_caller) => {
                self.vm.scratch = self.vm.caller.to_vec();

                println!("ext_caller: {}", hex::encode(&self.vm.scratch));

                Ok(None)
            }
            Some(SubstrateExternal::ext_balance) => {
                self.vm.scratch = self.accounts[&self.vm.address].1.to_le_bytes().to_vec();

                println!("ext_balance: {}", hex::encode(&self.vm.scratch));

                Ok(None)
            }
            Some(SubstrateExternal::ext_minimum_balance) => {
                self.vm.scratch = 500u128.to_le_bytes().to_vec();

                println!("ext_minimum_balance: {}", hex::encode(&self.vm.scratch));

                Ok(None)
            }
            Some(SubstrateExternal::ext_block_number) => {
                self.vm.scratch = 950_119_597u32.to_le_bytes().to_vec();

                println!("ext_block_number: {}", hex::encode(&self.vm.scratch));

                Ok(None)
            }
            Some(SubstrateExternal::ext_now) => {
                self.vm.scratch = 1594035638000u64.to_le_bytes().to_vec();

                println!("ext_now: {}", hex::encode(&self.vm.scratch));

                Ok(None)
            }
            Some(SubstrateExternal::ext_gas_left) => {
                self.vm.scratch = 2_224_097_461u64.to_le_bytes().to_vec();

                println!("ext_gas_left: {}", hex::encode(&self.vm.scratch));

                Ok(None)
            }
            Some(SubstrateExternal::ext_gas_price) => {
                self.vm.scratch = 59_541_253_813_967u128.to_le_bytes().to_vec();

                println!("ext_gas_price: {}", hex::encode(&self.vm.scratch));

                Ok(None)
            }
            Some(SubstrateExternal::ext_tombstone_deposit) => {
                self.vm.scratch = 93_603_701_976_053u128.to_le_bytes().to_vec();

                println!("ext_tombstone_deposit: {}", hex::encode(&self.vm.scratch));

                Ok(None)
            }
            Some(SubstrateExternal::ext_terminate) => {
                let address_ptr: u32 = args.nth_checked(0)?;
                let address_len: u32 = args.nth_checked(1)?;

                let mut address = [0u8; 32];

                if address_len != 32 {
                    panic!("ext_terminate: len = {}", address_len);
                }

                if let Err(e) = self.vm.memory.get_into(address_ptr, &mut address) {
                    panic!("ext_terminate: {}", e);
                }

                let remaining = self.accounts[&self.vm.address].1;

                self.accounts.get_mut(&address).unwrap().1 += remaining;

                println!("ext_terminate: {} {}", hex::encode(&address), remaining);

                self.accounts.remove(&self.vm.address);

                Err(Trap::new(TrapKind::Host(Box::new(HostCodeTerminate {}))))
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
            "ext_hash_sha2_256" => SubstrateExternal::ext_hash_sha2_256,
            "ext_hash_keccak_256" => SubstrateExternal::ext_hash_keccak_256,
            "ext_hash_blake2_128" => SubstrateExternal::ext_hash_blake2_128,
            "ext_hash_blake2_256" => SubstrateExternal::ext_hash_blake2_256,
            "ext_println" => SubstrateExternal::ext_println,
            "ext_call" => SubstrateExternal::ext_call,
            "ext_instantiate" => SubstrateExternal::ext_instantiate,
            "ext_value_transferred" => SubstrateExternal::ext_value_transferred,
            "ext_minimum_balance" => SubstrateExternal::ext_minimum_balance,
            "ext_random" => SubstrateExternal::ext_random,
            "ext_address" => SubstrateExternal::ext_address,
            "ext_balance" => SubstrateExternal::ext_balance,
            "ext_terminate" => SubstrateExternal::ext_terminate,
            "ext_block_number" => SubstrateExternal::ext_block_number,
            "ext_now" => SubstrateExternal::ext_now,
            "ext_gas_price" => SubstrateExternal::ext_gas_price,
            "ext_gas_left" => SubstrateExternal::ext_gas_left,
            "ext_caller" => SubstrateExternal::ext_caller,
            "ext_tombstone_deposit" => SubstrateExternal::ext_tombstone_deposit,
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

        let module = self.create_module(&self.accounts.get(&self.vm.address).unwrap().0);

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

    pub fn constructor_expect_return(&mut self, index: usize, expected_ret: i32, args: Vec<u8>) {
        let m = &self.abi.contract.constructors[index];

        let module = self.create_module(&self.accounts.get(&self.vm.address).unwrap().0);

        self.vm.scratch = m.selector().into_iter().chain(args).collect();

        if let Some(RuntimeValue::I32(ret)) = module
            .invoke_export("deploy", &[], self)
            .expect("failed to call function")
        {
            println!(
                "function_expected_return: got {} expected {}",
                ret, expected_ret
            );

            if expected_ret != ret {
                panic!("non one return")
            }
        }
    }

    pub fn function(&mut self, name: &str, args: Vec<u8>) {
        let m = self.abi.get_function(name).unwrap();

        let module = self.create_module(&self.accounts.get(&self.vm.address).unwrap().0);

        self.vm.scratch = m.selector().into_iter().chain(args).collect();

        if let Some(RuntimeValue::I32(ret)) = module
            .invoke_export("call", &[], self)
            .expect("failed to call function")
        {
            if ret != 0 {
                panic!(format!("non zero return: {}", ret));
            }
        }
    }

    pub fn function_expect_return(&mut self, name: &str, args: Vec<u8>, expected_ret: i32) {
        let m = self.abi.get_function(name).unwrap();

        let module = self.create_module(&self.accounts.get(&self.vm.address).unwrap().0);

        self.vm.scratch = m.selector().into_iter().chain(args).collect();

        if let Some(RuntimeValue::I32(ret)) = module
            .invoke_export("call", &[], self)
            .expect("failed to call function")
        {
            println!(
                "function_expected_return: got {} expected {}",
                ret, expected_ret
            );

            if expected_ret != ret {
                panic!("non one return")
            }
        }
    }

    pub fn raw_function(&mut self, input: Vec<u8>) {
        let module = self.create_module(&self.accounts.get(&self.vm.address).unwrap().0);

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

    pub fn raw_function_return(&mut self, expect_ret: i32, input: Vec<u8>) {
        let module = self.create_module(&self.accounts.get(&self.vm.address).unwrap().0);

        self.vm.scratch = input;

        if let Some(RuntimeValue::I32(ret)) = module
            .invoke_export("call", &[], self)
            .expect("failed to call function")
        {
            println!("got {} expected {}", ret, expect_ret);

            if ret != expect_ret {
                panic!("return not expected")
            }
        }
    }

    pub fn raw_constructor(&mut self, input: Vec<u8>) {
        let module = self.create_module(&self.accounts.get(&self.vm.address).unwrap().0);

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

    pub fn heap_verify(&self) {
        let memsize = self.vm.memory.current_size().0 as usize * 0x10000;
        println!("memory size:{}", memsize);
        let mut buf = Vec::new();
        buf.resize(memsize, 0);

        let mut current_elem = 0x10000;
        let mut last_elem = 0u32;

        loop {
            let next: u32 = self.vm.memory.get_value(current_elem).unwrap();
            let prev: u32 = self.vm.memory.get_value(current_elem + 4).unwrap();
            let length: u32 = self.vm.memory.get_value(current_elem + 8).unwrap();
            let allocated: u32 = self.vm.memory.get_value(current_elem + 12).unwrap();

            println!(
                "next:{:08x} prev:{:08x} length:{} allocated:{}",
                next, prev, length, allocated
            );

            let buf = self
                .vm
                .memory
                .get(current_elem + 16, length as usize)
                .unwrap();

            if allocated == 0 {
                println!("{:08x} {} not allocated", current_elem + 16, length);
            } else {
                println!("{:08x} {} allocated", current_elem + 16, length);

                assert_eq!(allocated & 0xffff, 1);

                for offset in (0..buf.len()).step_by(16) {
                    let mut hex = "\t".to_string();
                    let mut chars = "\t".to_string();
                    for i in 0..16 {
                        if offset + i >= buf.len() {
                            break;
                        }
                        let b = buf[offset + i];
                        hex.push_str(&format!(" {:02x}", b));
                        if b >= 0x20 && b <= 0x7e {
                            chars.push_str(&format!("  {}", b as char));
                        } else {
                            chars.push_str("   ");
                        }
                    }
                    println!("{}\n{}", hex, chars);
                }
            }

            assert_eq!(last_elem, prev);

            if next == 0 {
                break;
            }

            last_elem = current_elem;
            current_elem = next;
        }
    }
}

pub fn parse_and_resolve(src: &'static str, target: Target) -> ast::Namespace {
    let mut cache = FileCache::new();

    cache.set_file_contents("test.sol".to_string(), src.to_string());

    solang::parse_and_resolve("test.sol", &mut cache, target)
}

pub fn build_solidity(src: &'static str) -> TestRuntime {
    let mut cache = FileCache::new();

    cache.set_file_contents("test.sol".to_string(), src.to_string());

    let (res, ns) = compile(
        "test.sol",
        &mut cache,
        inkwell::OptimizationLevel::Default,
        Target::Substrate,
    );

    diagnostics::print_messages(&mut cache, &ns, false);

    assert!(!res.is_empty());

    let abistr = res[0].1.clone();
    let code = res[0].0.clone();
    let address = address_new();

    let mut t = TestRuntime {
        accounts: HashMap::new(),
        printbuf: String::new(),
        store: HashMap::new(),
        contracts: res,
        vm: VM::new(address, address_new(), 0),
        abi: abi::substrate::load(&abistr).unwrap(),
    };

    t.accounts.insert(address, (code, 0));

    t
}

pub fn first_error(errors: Vec<ast::Diagnostic>) -> String {
    match errors.iter().find(|m| m.level == ast::Level::Error) {
        Some(m) => m.message.to_owned(),
        None => panic!("no errors found"),
    }
}

pub fn first_warning(errors: Vec<ast::Diagnostic>) -> String {
    match errors.iter().find(|m| m.level == ast::Level::Warning) {
        Some(m) => m.message.to_owned(),
        None => panic!("no warnings found"),
    }
}

pub fn no_errors(errors: Vec<ast::Diagnostic>) {
    assert!(
        errors
            .iter()
            .filter(|m| m.level == ast::Level::Error)
            .count()
            == 0
    );
}

pub fn no_warnings_errors(errors: Vec<ast::Diagnostic>) {
    assert!(
        errors
            .iter()
            .filter(|m| m.level == ast::Level::Error || m.level == ast::Level::Warning)
            .count()
            == 0
    );
}
