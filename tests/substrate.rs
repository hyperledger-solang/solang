// SPDX-License-Identifier: Apache-2.0

// Create WASM virtual machine like substrate
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use rand::Rng;
use sha2::{Digest, Sha256};
use std::{collections::HashMap, ffi::OsStr, fmt, fmt::Write};
use tiny_keccak::{Hasher, Keccak};
use wasmi::memory_units::Pages;
use wasmi::*;

use solang::abi;
use solang::file_resolver::FileResolver;
use solang::{compile, Target};

mod substrate_tests;

type StorageKey = [u8; 32];
type Account = [u8; 32];

fn account_new() -> Account {
    let mut rng = rand::thread_rng();

    let mut a = [0u8; 32];

    rng.fill(&mut a[..]);

    a
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HostCodeTerminate {}

impl HostError for HostCodeTerminate {}

impl fmt::Display for HostCodeTerminate {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "seal_terminate")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HostCodeReturn(i32);

impl fmt::Display for HostCodeReturn {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "return {}", self.0)
    }
}

impl HostError for HostCodeReturn {}

#[derive(FromPrimitive)]
#[allow(non_camel_case_types)]
enum SubstrateExternal {
    seal_input = 0,
    seal_set_storage,
    seal_clear_storage,
    seal_get_storage,
    seal_return,
    seal_hash_keccak_256,
    seal_debug_message,
    seal_call,
    seal_instantiate,
    seal_value_transferred,
    seal_minimum_balance,
    seal_random,
    seal_address,
    seal_balance,
    seal_terminate,
    seal_hash_sha2_256,
    seal_hash_blake2_128,
    seal_hash_blake2_256,
    seal_block_number,
    seal_now,
    seal_weight_to_fee,
    seal_gas_left,
    seal_caller,
    seal_tombstone_deposit,
    seal_deposit_event,
    seal_transfer,
}

pub struct Event {
    topics: Vec<[u8; 32]>,
    data: Vec<u8>,
}

pub struct VirtualMachine {
    account: Account,
    caller: Account,
    memory: MemoryRef,
    input: Vec<u8>,
    pub output: Vec<u8>,
    pub value: u128,
}

impl VirtualMachine {
    fn new(account: Account, caller: Account, value: u128) -> Self {
        VirtualMachine {
            memory: MemoryInstance::alloc(Pages(16), Some(Pages(16))).unwrap(),
            input: Vec::new(),
            output: Vec::new(),
            account,
            caller,
            value,
        }
    }
}

pub struct Program {
    abi: abi::substrate::Abi,
    code: Vec<u8>,
}
pub struct MockSubstrate {
    pub store: HashMap<(Account, StorageKey), Vec<u8>>,
    pub programs: Vec<Program>,
    pub printbuf: String,
    pub accounts: HashMap<Account, (Vec<u8>, u128)>,
    pub current_program: usize,
    pub vm: VirtualMachine,
    pub events: Vec<Event>,
}

impl Externals for MockSubstrate {
    #[allow(clippy::cognitive_complexity)]
    fn invoke_index(
        &mut self,
        index: usize,
        args: RuntimeArgs,
    ) -> Result<Option<RuntimeValue>, Trap> {
        macro_rules! set_seal_value {
            ($name:literal, $dest_ptr:expr, $len_ptr:expr, $buf:expr) => {{
                println!("{}: {}", $name, hex::encode($buf));

                let len = self
                    .vm
                    .memory
                    .get_value::<u32>($len_ptr)
                    .expect(&format!("{} len_ptr should be valid", $name));

                assert!(
                    (len as usize) >= $buf.len(),
                    "{} input is {} buffer is {}",
                    $name,
                    $buf.len(),
                    len
                );

                if let Err(e) = self.vm.memory.set($dest_ptr, $buf) {
                    panic!("{}: {}", $name, e);
                }

                self.vm
                    .memory
                    .set_value($len_ptr, $buf.len() as u32)
                    .expect(&format!("{} len_ptr should be valid", $name));
            }};
        }

        match FromPrimitive::from_usize(index) {
            Some(SubstrateExternal::seal_input) => {
                let dest_ptr: u32 = args.nth_checked(0)?;
                let len_ptr: u32 = args.nth_checked(1)?;

                let len = self
                    .vm
                    .memory
                    .get_value::<u32>(len_ptr)
                    .expect("seal_input len_ptr should be valid");

                assert!(
                    (len as usize) >= self.vm.input.len(),
                    "input is {} seal_input buffer {}",
                    self.vm.input.len(),
                    len
                );

                if let Err(e) = self.vm.memory.set(dest_ptr, &self.vm.input) {
                    panic!("seal_input: {}", e);
                }

                self.vm
                    .memory
                    .set_value(len_ptr, self.vm.input.len() as u32)
                    .expect("seal_input len_ptr should be valid");

                Ok(None)
            }
            Some(SubstrateExternal::seal_get_storage) => {
                assert_eq!(args.len(), 3);

                let key_ptr: u32 = args.nth_checked(0)?;
                let dest_ptr: u32 = args.nth_checked(1)?;
                let len_ptr: u32 = args.nth_checked(2)?;

                let mut key: StorageKey = [0; 32];

                if let Err(e) = self.vm.memory.get_into(key_ptr, &mut key) {
                    panic!("seal_get_storage: {}", e);
                }

                if let Some(value) = self.store.get(&(self.vm.account, key)) {
                    println!("seal_get_storage: {:?} = {:?}", key, value);

                    let len = self
                        .vm
                        .memory
                        .get_value::<u32>(len_ptr)
                        .expect("seal_get_storage len_ptr should be valid");

                    assert!(
                        (len as usize) >= value.len(),
                        "seal_get_storage buffer is too small"
                    );

                    if let Err(e) = self.vm.memory.set(dest_ptr, value) {
                        panic!("seal_get_storage: {}", e);
                    }

                    self.vm
                        .memory
                        .set_value(len_ptr, value.len() as u32)
                        .expect("seal_get_storage len_ptr should be valid");

                    Ok(Some(RuntimeValue::I32(0)))
                } else {
                    println!("seal_get_storage: {:?} = nil", key);
                    Ok(Some(RuntimeValue::I32(1)))
                }
            }
            Some(SubstrateExternal::seal_clear_storage) => {
                let key_ptr: u32 = args.nth_checked(0)?;

                let mut key: StorageKey = [0; 32];

                if let Err(e) = self.vm.memory.get_into(key_ptr, &mut key) {
                    panic!("seal_clear_storage: {}", e);
                }

                println!("seal_clear_storage: {:?}", key);
                self.store.remove(&(self.vm.account, key));

                Ok(None)
            }
            Some(SubstrateExternal::seal_set_storage) => {
                assert_eq!(args.len(), 3);

                let key_ptr: u32 = args.nth_checked(0)?;
                let data_ptr: u32 = args.nth_checked(1)?;
                let len: u32 = args.nth_checked(2)?;

                let mut key: StorageKey = [0; 32];

                if let Err(e) = self.vm.memory.get_into(key_ptr, &mut key) {
                    panic!("seal_set_storage: {}", e);
                }

                let mut data = Vec::new();
                data.resize(len as usize, 0u8);

                if let Err(e) = self.vm.memory.get_into(data_ptr, &mut data) {
                    panic!("seal_set_storage: {}", e);
                }
                println!("seal_set_storage: {:?} = {:?}", key, data);

                self.store.insert((self.vm.account, key), data);

                Ok(None)
            }
            Some(SubstrateExternal::seal_hash_keccak_256) => {
                let data_ptr: u32 = args.nth_checked(0)?;
                let len: u32 = args.nth_checked(1)?;
                let out_ptr: u32 = args.nth_checked(2)?;

                let mut data = Vec::new();

                data.resize(len as usize, 0);

                if let Err(e) = self.vm.memory.get_into(data_ptr, &mut data) {
                    panic!("seal_hash_keccak_256: {}", e);
                }

                let mut hasher = Keccak::v256();
                let mut hash = [0u8; 32];
                hasher.update(&data);
                hasher.finalize(&mut hash);

                println!(
                    "seal_hash_keccak_256: {} = {}",
                    hex::encode(data),
                    hex::encode(hash)
                );

                if let Err(e) = self.vm.memory.set(out_ptr, &hash) {
                    panic!("seal_hash_keccak_256: {}", e);
                }

                Ok(None)
            }
            Some(SubstrateExternal::seal_hash_sha2_256) => {
                let data_ptr: u32 = args.nth_checked(0)?;
                let len: u32 = args.nth_checked(1)?;
                let out_ptr: u32 = args.nth_checked(2)?;

                let mut data = Vec::new();

                data.resize(len as usize, 0);

                if let Err(e) = self.vm.memory.get_into(data_ptr, &mut data) {
                    panic!("seal_hash_sha2_256: {}", e);
                }

                let mut hasher = Sha256::new();

                hasher.update(&data);
                let hash = hasher.finalize();

                println!(
                    "seal_hash_sha2_256: {} = {}",
                    hex::encode(data),
                    hex::encode(hash)
                );

                if let Err(e) = self.vm.memory.set(out_ptr, &hash) {
                    panic!("seal_hash_sha2_256: {}", e);
                }

                Ok(None)
            }
            Some(SubstrateExternal::seal_hash_blake2_128) => {
                let data_ptr: u32 = args.nth_checked(0)?;
                let len: u32 = args.nth_checked(1)?;
                let out_ptr: u32 = args.nth_checked(2)?;

                let mut data = Vec::new();

                data.resize(len as usize, 0);

                if let Err(e) = self.vm.memory.get_into(data_ptr, &mut data) {
                    panic!("seal_hash_blake2_128: {}", e);
                }
                let hash = blake2_rfc::blake2b::blake2b(16, &[], &data);

                println!(
                    "seal_hash_blake2_128: {} = {}",
                    hex::encode(data),
                    hex::encode(hash)
                );

                if let Err(e) = self.vm.memory.set(out_ptr, hash.as_bytes()) {
                    panic!("seal_hash_blake2_128: {}", e);
                }

                Ok(None)
            }
            Some(SubstrateExternal::seal_hash_blake2_256) => {
                let data_ptr: u32 = args.nth_checked(0)?;
                let len: u32 = args.nth_checked(1)?;
                let out_ptr: u32 = args.nth_checked(2)?;

                let mut data = Vec::new();

                data.resize(len as usize, 0);

                if let Err(e) = self.vm.memory.get_into(data_ptr, &mut data) {
                    panic!("seal_hash_blake2_256: {}", e);
                }

                let hash = blake2_rfc::blake2b::blake2b(32, &[], &data);

                println!(
                    "seal_hash_blake2_256: {} = {}",
                    hex::encode(data),
                    hex::encode(hash)
                );

                if let Err(e) = self.vm.memory.set(out_ptr, hash.as_bytes()) {
                    panic!("seal_hash_blake2_256: {}", e);
                }

                Ok(None)
            }
            Some(SubstrateExternal::seal_return) => {
                let flags: i32 = args.nth_checked(0)?;
                let data_ptr: u32 = args.nth_checked(1)?;
                let len: u32 = args.nth_checked(2)?;

                self.vm.output.resize(len as usize, 0u8);

                if let Err(e) = self.vm.memory.get_into(data_ptr, &mut self.vm.output) {
                    panic!("seal_return: {}", e);
                }

                match flags {
                    0 | 1 => Err(Trap::new(TrapKind::Host(Box::new(HostCodeReturn(flags))))),
                    _ => panic!("seal_return flag {} not valid", flags),
                }
            }
            Some(SubstrateExternal::seal_debug_message) => {
                let data_ptr: u32 = args.nth_checked(0)?;
                let len: u32 = args.nth_checked(1)?;

                let mut buf = Vec::new();
                buf.resize(len as usize, 0u8);

                if let Err(e) = self.vm.memory.get_into(data_ptr, &mut buf) {
                    panic!("seal_debug_message: {}", e);
                }

                let s = String::from_utf8(buf).expect("seal_debug_message: Invalid UFT8");

                println!("seal_debug_message: {}", s);

                self.printbuf.push_str(&s);

                Ok(Some(RuntimeValue::I32(0)))
            }
            Some(SubstrateExternal::seal_random) => {
                let data_ptr: u32 = args.nth_checked(0)?;
                let len: u32 = args.nth_checked(1)?;
                let dest_ptr: u32 = args.nth_checked(2)?;
                let len_ptr: u32 = args.nth_checked(3)?;

                let mut buf = Vec::new();
                buf.resize(len as usize, 0u8);

                if let Err(e) = self.vm.memory.get_into(data_ptr, &mut buf) {
                    panic!("seal_random: {}", e);
                }

                let mut hash = [0u8; 32];

                hash.copy_from_slice(blake2_rfc::blake2b::blake2b(32, &[], &buf).as_bytes());

                println!("seal_random: {} {}", hex::encode(buf), hex::encode(&hash));

                let len = self
                    .vm
                    .memory
                    .get_value::<u32>(len_ptr)
                    .expect("seal_random len_ptr should be valid");

                assert!(
                    (len as usize) >= hash.len(),
                    "seal_random dest buffer is too small"
                );

                if let Err(e) = self.vm.memory.set(dest_ptr, &hash) {
                    panic!("seal_random: {}", e);
                }

                self.vm
                    .memory
                    .set_value(len_ptr, hash.len() as u32)
                    .expect("seal_random len_ptr should be valid");

                Ok(None)
            }
            Some(SubstrateExternal::seal_call) => {
                let account_ptr: u32 = args.nth_checked(0)?;
                let account_len: u32 = args.nth_checked(1)?;
                //let gas: u64 = args.nth_checked(2)?;
                let value_ptr: u32 = args.nth_checked(3)?;
                let value_len: u32 = args.nth_checked(4)?;
                let input_ptr: u32 = args.nth_checked(5)?;
                let input_len: u32 = args.nth_checked(6)?;
                let output_ptr: u32 = args.nth_checked(7)?;
                let output_len_ptr: u32 = args.nth_checked(8)?;

                let mut account = [0u8; 32];

                assert!(account_len == 32, "seal_call: len = {}", account_len);

                if let Err(e) = self.vm.memory.get_into(account_ptr, &mut account) {
                    panic!("seal_call: {}", e);
                }

                let mut value = [0u8; 16];

                assert!(value_len == 16, "seal_call: len = {}", value_len);

                if let Err(e) = self.vm.memory.get_into(value_ptr, &mut value) {
                    panic!("seal_call: {}", e);
                }

                let value = u128::from_le_bytes(value);

                if !self.accounts.contains_key(&account) {
                    // substrate would return NotCallable
                    return Ok(Some(RuntimeValue::I32(0x8)));
                }

                let mut input = Vec::new();
                input.resize(input_len as usize, 0u8);

                if let Err(e) = self.vm.memory.get_into(input_ptr, &mut input) {
                    panic!("seal_call: {}", e);
                }

                println!(
                    "seal_call: account={} input={}",
                    hex::encode(account),
                    hex::encode(&input)
                );

                let mut vm = VirtualMachine::new(account, self.vm.account, value);

                std::mem::swap(&mut self.vm, &mut vm);

                let module = self.create_module(&self.accounts.get(&self.vm.account).unwrap().0);

                self.vm.input = input;

                let ret = module.invoke_export("call", &[], self);

                let ret = match ret {
                    Err(wasmi::Error::Trap(trap)) => match trap.kind() {
                        TrapKind::Host(host_error) => {
                            if let Some(ret) = host_error.downcast_ref::<HostCodeReturn>() {
                                Some(RuntimeValue::I32(ret.0))
                            } else if host_error.downcast_ref::<HostCodeTerminate>().is_some() {
                                Some(RuntimeValue::I32(1))
                            } else {
                                return Err(trap);
                            }
                        }
                        _ => {
                            return Err(trap);
                        }
                    },
                    Ok(v) => v,
                    Err(e) => panic!("fail to invoke call: {}", e),
                };

                let output = self.vm.output.clone();

                std::mem::swap(&mut self.vm, &mut vm);

                println!("seal_call ret={:?} buf={}", ret, hex::encode(&output));

                if let Some(acc) = self.accounts.get_mut(&vm.account) {
                    acc.1 += vm.value;
                }

                set_seal_value!("seal_call return buf", output_ptr, output_len_ptr, &output);

                Ok(ret)
            }
            Some(SubstrateExternal::seal_transfer) => {
                let account_ptr: u32 = args.nth_checked(0)?;
                let account_len: u32 = args.nth_checked(1)?;
                let value_ptr: u32 = args.nth_checked(2)?;
                let value_len: u32 = args.nth_checked(3)?;

                let mut account = [0u8; 32];

                assert!(account_len == 32, "seal_transfer: len = {}", account_len);

                if let Err(e) = self.vm.memory.get_into(account_ptr, &mut account) {
                    panic!("seal_transfer: {}", e);
                }

                let mut value = [0u8; 16];

                assert!(value_len == 16, "seal_transfer: len = {}", value_len);

                if let Err(e) = self.vm.memory.get_into(value_ptr, &mut value) {
                    panic!("seal_transfer: {}", e);
                }

                let value = u128::from_le_bytes(value);

                if !self.accounts.contains_key(&account) {
                    // substrate would return TransferFailed
                    return Ok(Some(RuntimeValue::I32(0x5)));
                }

                if let Some(acc) = self.accounts.get_mut(&account) {
                    acc.1 += value;
                }

                Ok(Some(RuntimeValue::I32(0)))
            }
            Some(SubstrateExternal::seal_instantiate) => {
                let codehash_ptr: u32 = args.nth_checked(0)?;
                let codehash_len: u32 = args.nth_checked(1)?;
                //let gas: u64 = args.nth_checked(2)?;
                let value_ptr: u32 = args.nth_checked(3)?;
                let value_len: u32 = args.nth_checked(4)?;
                let input_ptr: u32 = args.nth_checked(5)?;
                let input_len: u32 = args.nth_checked(6)?;
                let account_ptr: u32 = args.nth_checked(7)?;
                let account_len_ptr: u32 = args.nth_checked(8)?;
                let output_ptr: u32 = args.nth_checked(9)?;
                let output_len_ptr: u32 = args.nth_checked(10)?;
                let salt_ptr: u32 = args.nth_checked(11)?;
                let salt_len: u32 = args.nth_checked(12)?;

                let mut codehash = [0u8; 32];

                assert!(
                    codehash_len == 32,
                    "seal_instantiate: len = {}",
                    codehash_len
                );

                if let Err(e) = self.vm.memory.get_into(codehash_ptr, &mut codehash) {
                    panic!("seal_instantiate: {}", e);
                }

                let mut value = [0u8; 16];

                assert!(value_len == 16, "seal_instantiate: len = {}", value_len);

                if let Err(e) = self.vm.memory.get_into(value_ptr, &mut value) {
                    panic!("seal_instantiate: {}", e);
                }

                let value = u128::from_le_bytes(value);

                let mut input = Vec::new();
                input.resize(input_len as usize, 0u8);

                if let Err(e) = self.vm.memory.get_into(input_ptr, &mut input) {
                    panic!("seal_instantiate: {}", e);
                }

                let mut salt = Vec::new();
                salt.resize(salt_len as usize, 0u8);

                if let Err(e) = self.vm.memory.get_into(salt_ptr, &mut salt) {
                    panic!("seal_instantiate: {}", e);
                }

                println!(
                    "seal_instantiate value:{} input={} salt={}",
                    value,
                    hex::encode(&input),
                    hex::encode(&salt),
                );

                let mut account = [0u8; 32];

                let hash_data: Vec<u8> = input.iter().chain(salt.iter()).cloned().collect();

                account
                    .copy_from_slice(blake2_rfc::blake2b::blake2b(32, &[], &hash_data).as_bytes());

                if self.accounts.contains_key(&account) {
                    // substrate would return TRAP_RETURN_CODE (0x0100)
                    return Ok(Some(RuntimeValue::I32(0x100)));
                }

                let program = self
                    .programs
                    .iter()
                    .find(|program| {
                        blake2_rfc::blake2b::blake2b(32, &[], &program.code).as_bytes() == codehash
                    })
                    .expect("codehash not found");

                self.accounts.insert(account, (program.code.clone(), 0));

                let mut input = Vec::new();
                input.resize(input_len as usize, 0u8);

                if let Err(e) = self.vm.memory.get_into(input_ptr, &mut input) {
                    panic!("seal_instantiate: {}", e);
                }

                let mut vm = VirtualMachine::new(account, self.vm.account, value);

                std::mem::swap(&mut self.vm, &mut vm);

                let module = self.create_module(&program.code);

                self.vm.input = input;

                let ret = match module.invoke_export("deploy", &[], self) {
                    Err(wasmi::Error::Trap(trap)) => match trap.kind() {
                        TrapKind::Host(host_error) => {
                            if let Some(ret) = host_error.downcast_ref::<HostCodeReturn>() {
                                Some(RuntimeValue::I32(ret.0))
                            } else {
                                return Err(trap);
                            }
                        }
                        _ => {
                            return Err(trap);
                        }
                    },
                    Ok(v) => v,
                    Err(e) => panic!("fail to invoke deploy: {}", e),
                };

                let output = self.vm.output.clone();

                std::mem::swap(&mut self.vm, &mut vm);

                set_seal_value!(
                    "seal_instantiate output",
                    output_ptr,
                    output_len_ptr,
                    &output
                );

                if let Some(RuntimeValue::I32(0)) = ret {
                    self.accounts.get_mut(&vm.account).unwrap().1 += vm.value;
                    set_seal_value!(
                        "seal_instantiate account",
                        account_ptr,
                        account_len_ptr,
                        &account
                    );
                }

                println!("seal_instantiate ret:{:?}", ret);

                Ok(ret)
            }
            Some(SubstrateExternal::seal_value_transferred) => {
                let dest_ptr: u32 = args.nth_checked(0)?;
                let len_ptr: u32 = args.nth_checked(1)?;

                let scratch = self.vm.value.to_le_bytes();

                set_seal_value!("seal_value_transferred", dest_ptr, len_ptr, &scratch);

                Ok(None)
            }
            Some(SubstrateExternal::seal_address) => {
                let dest_ptr: u32 = args.nth_checked(0)?;
                let len_ptr: u32 = args.nth_checked(1)?;

                let scratch = self.vm.account;

                set_seal_value!("seal_address", dest_ptr, len_ptr, &scratch);

                Ok(None)
            }
            Some(SubstrateExternal::seal_caller) => {
                let dest_ptr: u32 = args.nth_checked(0)?;

                let len_ptr: u32 = args.nth_checked(1)?;
                let scratch = self.vm.caller;

                set_seal_value!("seal_caller", dest_ptr, len_ptr, &scratch);

                Ok(None)
            }
            Some(SubstrateExternal::seal_balance) => {
                let dest_ptr: u32 = args.nth_checked(0)?;
                let len_ptr: u32 = args.nth_checked(1)?;

                let scratch = self.accounts[&self.vm.account].1.to_le_bytes();

                set_seal_value!("seal_balance", dest_ptr, len_ptr, &scratch);

                Ok(None)
            }
            Some(SubstrateExternal::seal_minimum_balance) => {
                let dest_ptr: u32 = args.nth_checked(0)?;
                let len_ptr: u32 = args.nth_checked(1)?;

                let scratch = 500u128.to_le_bytes();

                set_seal_value!("seal_minimum_balance", dest_ptr, len_ptr, &scratch);

                Ok(None)
            }
            Some(SubstrateExternal::seal_block_number) => {
                let dest_ptr: u32 = args.nth_checked(0)?;
                let len_ptr: u32 = args.nth_checked(1)?;

                let scratch = 950_119_597u32.to_le_bytes();

                set_seal_value!("seal_block_number", dest_ptr, len_ptr, &scratch);

                Ok(None)
            }
            Some(SubstrateExternal::seal_now) => {
                let dest_ptr: u32 = args.nth_checked(0)?;
                let len_ptr: u32 = args.nth_checked(1)?;

                let scratch = 1594035638000u64.to_le_bytes();

                set_seal_value!("seal_now", dest_ptr, len_ptr, &scratch);

                Ok(None)
            }
            Some(SubstrateExternal::seal_gas_left) => {
                let dest_ptr: u32 = args.nth_checked(0)?;
                let len_ptr: u32 = args.nth_checked(1)?;

                let scratch = 2_224_097_461u64.to_le_bytes();

                set_seal_value!("seal_gas_left", dest_ptr, len_ptr, &scratch);

                Ok(None)
            }
            Some(SubstrateExternal::seal_weight_to_fee) => {
                let units: u64 = args.nth_checked(0)?;
                let dest_ptr: u32 = args.nth_checked(1)?;
                let len_ptr: u32 = args.nth_checked(2)?;

                let scratch = (59_541_253_813_967u128 * units as u128).to_le_bytes();

                set_seal_value!("seal_weight_to_fee", dest_ptr, len_ptr, &scratch);

                Ok(None)
            }
            Some(SubstrateExternal::seal_tombstone_deposit) => {
                let dest_ptr: u32 = args.nth_checked(0)?;
                let len_ptr: u32 = args.nth_checked(1)?;

                let scratch = 93_603_701_976_053u128.to_le_bytes();

                set_seal_value!("seal_tombstone_deposit", dest_ptr, len_ptr, &scratch);

                Ok(None)
            }
            Some(SubstrateExternal::seal_terminate) => {
                let account_ptr: u32 = args.nth_checked(0)?;
                let account_len: u32 = args.nth_checked(1)?;

                let mut account = [0u8; 32];

                assert!(account_len == 32, "seal_terminate: len = {}", account_len);

                if let Err(e) = self.vm.memory.get_into(account_ptr, &mut account) {
                    panic!("seal_terminate: {}", e);
                }

                let remaining = self.accounts[&self.vm.account].1;

                self.accounts.get_mut(&account).unwrap().1 += remaining;

                println!("seal_terminate: {} {}", hex::encode(&account), remaining);

                self.accounts.remove(&self.vm.account);

                Err(Trap::new(TrapKind::Host(Box::new(HostCodeTerminate {}))))
            }
            Some(SubstrateExternal::seal_deposit_event) => {
                let mut topic_ptr: u32 = args.nth_checked(0)?;
                let topic_len: u32 = args.nth_checked(1)?;
                let data_ptr: u32 = args.nth_checked(2)?;
                let data_len: u32 = args.nth_checked(3)?;

                let mut topics = Vec::new();

                if topic_len != 0 {
                    assert_eq!(topic_len % 32, 1);
                    assert_eq!((topic_len - 1) % 32, 0);

                    let mut vec_length = [0u8];

                    if let Err(e) = self.vm.memory.get_into(topic_ptr, &mut vec_length) {
                        panic!("seal_deposit_event: topic: {}", e);
                    }

                    println!("topic_len: {} first byte: {}", topic_len, vec_length[0]);
                    assert_eq!(vec_length[0] as u32, (topic_len - 1) / 8);

                    topic_ptr += 1;
                }

                for _ in 0..topic_len / 32 {
                    let mut topic = [0u8; 32];
                    if let Err(e) = self.vm.memory.get_into(topic_ptr, &mut topic) {
                        panic!("seal_deposit_event: topic: {}", e);
                    }
                    topics.push(topic);
                    topic_ptr += 32;
                }

                let mut data = Vec::new();
                data.resize(data_len as usize, 0);

                if let Err(e) = self.vm.memory.get_into(data_ptr, &mut data) {
                    panic!("seal_deposit_event: data: {}", e);
                }

                println!(
                    "seal_deposit_event: topic: {} data: {}",
                    topics
                        .iter()
                        .map(|t| hex::encode(&t))
                        .collect::<Vec<String>>()
                        .join(" "),
                    hex::encode(&data)
                );

                self.events.push(Event { topics, data });

                Ok(None)
            }
            _ => panic!("external {} unknown", index),
        }
    }
}

impl ModuleImportResolver for MockSubstrate {
    fn resolve_func(&self, field_name: &str, signature: &Signature) -> Result<FuncRef, Error> {
        let index = match field_name {
            "seal_input" => SubstrateExternal::seal_input,
            "seal_get_storage" => SubstrateExternal::seal_get_storage,
            "seal_set_storage" => SubstrateExternal::seal_set_storage,
            "seal_clear_storage" => SubstrateExternal::seal_clear_storage,
            "seal_return" => SubstrateExternal::seal_return,
            "seal_hash_sha2_256" => SubstrateExternal::seal_hash_sha2_256,
            "seal_hash_keccak_256" => SubstrateExternal::seal_hash_keccak_256,
            "seal_hash_blake2_128" => SubstrateExternal::seal_hash_blake2_128,
            "seal_hash_blake2_256" => SubstrateExternal::seal_hash_blake2_256,
            "seal_debug_message" => SubstrateExternal::seal_debug_message,
            "seal_call" => SubstrateExternal::seal_call,
            "seal_instantiate" => SubstrateExternal::seal_instantiate,
            "seal_value_transferred" => SubstrateExternal::seal_value_transferred,
            "seal_minimum_balance" => SubstrateExternal::seal_minimum_balance,
            "seal_random" => SubstrateExternal::seal_random,
            "seal_address" => SubstrateExternal::seal_address,
            "seal_balance" => SubstrateExternal::seal_balance,
            "seal_terminate" => SubstrateExternal::seal_terminate,
            "seal_block_number" => SubstrateExternal::seal_block_number,
            "seal_now" => SubstrateExternal::seal_now,
            "seal_weight_to_fee" => SubstrateExternal::seal_weight_to_fee,
            "seal_gas_left" => SubstrateExternal::seal_gas_left,
            "seal_caller" => SubstrateExternal::seal_caller,
            "seal_tombstone_deposit" => SubstrateExternal::seal_tombstone_deposit,
            "seal_deposit_event" => SubstrateExternal::seal_deposit_event,
            "seal_transfer" => SubstrateExternal::seal_transfer,
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

impl MockSubstrate {
    fn create_module(&self, code: &[u8]) -> ModuleRef {
        let module = Module::from_buffer(&code).expect("parse wasm should work");

        ModuleInstance::new(
            &module,
            &ImportsBuilder::new()
                .with_resolver("env", self)
                .with_resolver("seal0", self),
        )
        .expect("Failed to instantiate module")
        .run_start(&mut NopExternals)
        .expect("Failed to run start function in module")
    }

    fn invoke_deploy(&mut self, module: ModuleRef) -> Option<RuntimeValue> {
        match module.invoke_export("deploy", &[], self) {
            Err(wasmi::Error::Trap(trap)) => match trap.kind() {
                TrapKind::Host(host_error) => {
                    if let Some(ret) = host_error.downcast_ref::<HostCodeReturn>() {
                        Some(RuntimeValue::I32(ret.0))
                    } else {
                        panic!("did not go as planned");
                    }
                }
                _ => panic!("fail to invoke deploy: {}", trap),
            },
            Ok(v) => v,
            Err(e) => panic!("fail to invoke deploy: {}", e),
        }
    }

    fn invoke_call(&mut self, module: ModuleRef) -> Option<RuntimeValue> {
        match module.invoke_export("call", &[], self) {
            Err(wasmi::Error::Trap(trap)) => match trap.kind() {
                TrapKind::Host(host_error) => {
                    if let Some(ret) = host_error.downcast_ref::<HostCodeReturn>() {
                        Some(RuntimeValue::I32(ret.0))
                    } else if host_error.downcast_ref::<HostCodeTerminate>().is_some() {
                        Some(RuntimeValue::I32(1))
                    } else {
                        panic!("did not go as planned");
                    }
                }
                _ => panic!("fail to invoke call: {}", trap),
            },
            Ok(v) => v,
            Err(e) => panic!("fail to invoke call: {}", e),
        }
    }

    pub fn set_program(&mut self, index: usize) {
        let account = account_new();

        let code = self.programs[index].code.clone();
        self.accounts.insert(account, (code, 0));
        self.vm = VirtualMachine::new(account, account_new(), 0);

        self.current_program = index;
    }

    pub fn constructor(&mut self, index: usize, args: Vec<u8>) {
        let m = &self.programs[self.current_program].abi.spec.constructors[index];

        let module = self.create_module(&self.accounts.get(&self.vm.account).unwrap().0);

        self.vm.input = m.selector().into_iter().chain(args).collect();

        let ret = self.invoke_deploy(module);

        if let Some(RuntimeValue::I32(ret)) = ret {
            if ret != 0 {
                panic!("non zero return")
            }
        }
    }

    pub fn constructor_expect_return(&mut self, index: usize, expected_ret: i32, args: Vec<u8>) {
        let m = &self.programs[self.current_program].abi.spec.constructors[index];

        let module = self.create_module(&self.accounts.get(&self.vm.account).unwrap().0);

        self.vm.input = m.selector().into_iter().chain(args).collect();

        let ret = self.invoke_deploy(module);

        if let Some(RuntimeValue::I32(ret)) = ret {
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
        let m = self.programs[self.current_program]
            .abi
            .get_function(name)
            .unwrap();

        let module = self.create_module(&self.accounts.get(&self.vm.account).unwrap().0);

        self.vm.input = m.selector().into_iter().chain(args).collect();

        println!("input:{}", hex::encode(&self.vm.input));

        if let Some(RuntimeValue::I32(ret)) = self.invoke_call(module) {
            assert!(ret == 0, "non zero return: {}", ret);
        }
    }

    pub fn function_expect_failure(&mut self, name: &str, args: Vec<u8>) {
        let m = self.programs[self.current_program]
            .abi
            .get_function(name)
            .unwrap();

        let module = self.create_module(&self.accounts.get(&self.vm.account).unwrap().0);

        self.vm.input = m.selector().into_iter().chain(args).collect();

        match module.invoke_export("call", &[], self) {
            Err(wasmi::Error::Trap(trap)) => match trap.kind() {
                TrapKind::Unreachable => (),
                _ => panic!("trap: {:?}", trap),
            },
            Err(err) => {
                panic!("unexpected error: {:?}", err);
            }
            Ok(v) => {
                panic!("unexpected return value: {:?}", v);
            }
        }
    }

    pub fn raw_function(&mut self, input: Vec<u8>) {
        let module = self.create_module(&self.accounts.get(&self.vm.account).unwrap().0);

        self.vm.input = input;

        if let Some(RuntimeValue::I32(ret)) = self.invoke_call(module) {
            if ret != 0 {
                panic!("non zero return")
            }
        }
    }

    pub fn raw_function_failure(&mut self, input: Vec<u8>) {
        let module = self.create_module(&self.accounts.get(&self.vm.account).unwrap().0);

        self.vm.input = input;

        match module.invoke_export("call", &[], self) {
            Err(wasmi::Error::Trap(trap)) => match trap.kind() {
                TrapKind::Unreachable => (),
                _ => panic!("trap: {:?}", trap),
            },
            Err(err) => {
                panic!("unexpected error: {:?}", err);
            }
            Ok(v) => {
                panic!("unexpected return value: {:?}", v);
            }
        }
    }

    pub fn raw_constructor(&mut self, input: Vec<u8>) {
        let module = self.create_module(&self.accounts.get(&self.vm.account).unwrap().0);

        self.vm.input = input;

        if let Some(RuntimeValue::I32(ret)) = self.invoke_deploy(module) {
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

            let mut buf = vec![0u8; length as usize];

            self.vm
                .memory
                .get_into(current_elem + 16, &mut buf)
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
                        write!(hex, " {:02x}", b).unwrap();
                        if b.is_ascii() && !b.is_ascii_control() {
                            write!(chars, "  {}", b as char).unwrap();
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

pub fn build_solidity(src: &str) -> MockSubstrate {
    let mut cache = FileResolver::new();

    cache.set_file_contents("test.sol", src.to_string());

    let (res, ns) = compile(
        OsStr::new("test.sol"),
        &mut cache,
        inkwell::OptimizationLevel::Default,
        Target::default_substrate(),
        false,
    );

    ns.print_diagnostics_in_plain(&cache, false);

    assert!(!ns.diagnostics.any_errors());

    assert!(!res.is_empty());

    let programs: Vec<Program> = res
        .iter()
        .map(|res| Program {
            code: res.0.clone(),
            abi: abi::substrate::load(&res.1).unwrap(),
        })
        .collect();

    let mut accounts = HashMap::new();

    let account = account_new();

    accounts.insert(account, (programs[0].code.clone(), 0));

    let vm = VirtualMachine::new(account, account_new(), 0);

    MockSubstrate {
        accounts,
        printbuf: String::new(),
        store: HashMap::new(),
        programs,
        vm,
        current_program: 0,
        events: Vec::new(),
    }
}

pub fn build_solidity_with_overflow_check(src: &'static str) -> MockSubstrate {
    let mut cache = FileResolver::new();

    cache.set_file_contents("test.sol", src.to_string());

    let (res, ns) = compile(
        OsStr::new("test.sol"),
        &mut cache,
        inkwell::OptimizationLevel::Default,
        Target::default_substrate(),
        true,
    );

    ns.print_diagnostics_in_plain(&cache, false);

    assert!(!res.is_empty());

    let programs: Vec<Program> = res
        .iter()
        .map(|res| Program {
            code: res.0.clone(),
            abi: abi::substrate::load(&res.1).unwrap(),
        })
        .collect();

    let mut accounts = HashMap::new();

    let account = account_new();

    accounts.insert(account, (programs[0].code.clone(), 0));

    let vm = VirtualMachine::new(account, account_new(), 0);

    MockSubstrate {
        accounts,
        printbuf: String::new(),
        store: HashMap::new(),
        programs,
        vm,
        current_program: 0,
        events: Vec::new(),
    }
}
