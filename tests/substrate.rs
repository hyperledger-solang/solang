// SPDX-License-Identifier: Apache-2.0

/// Mock runtime for the contracts pallet.
use blake2_rfc::blake2b::blake2b;
use contract_metadata::ContractMetadata;
use ink_metadata::InkProject;
use ink_primitives::Hash;
use parity_scale_codec::Decode;
use sha2::{Digest, Sha256};
use std::{collections::HashMap, ffi::OsStr, fmt, fmt::Write};
use tiny_keccak::{Hasher, Keccak};
use wasmi::core::{HostError, Trap, TrapCode};
use wasmi::{Engine, Error, Instance, Linker, Memory, MemoryType, Module, Store};

use solang::file_resolver::FileResolver;
use solang::{compile, Target};

use wasm_host_derive::wasm_host;

mod substrate_tests;

type StorageKey = [u8; 32];
type Account = [u8; 32];
type Storage = HashMap<StorageKey, Vec<u8>>;

#[derive(Clone)]
pub struct Contract {
    messages: HashMap<String, Vec<u8>>,
    constructors: Vec<Vec<u8>>,
    code_hash: [u8; 32],
    address: Account,
    code: Vec<u8>,
    storage: Storage,
    value: u128,
}

impl Contract {
    fn new(abi: &str, code: &[u8]) -> Self {
        let abi = load_abi(abi);
        let messages = abi
            .spec()
            .messages()
            .iter()
            .map(|f| (f.label().to_string(), f.selector().to_bytes().to_vec()))
            .collect();
        let constructors = abi
            .spec()
            .constructors()
            .iter()
            .map(|f| f.selector().to_bytes().to_vec())
            .collect::<Vec<_>>();

        Self {
            messages,
            constructors,
            code_hash: blake2b(32, &[], code).as_bytes().try_into().unwrap(),
            address: rand::random(),
            code: code.to_vec(),
            storage: HashMap::new(),
            value: 0,
        }
    }

    fn create_instance(&self, salt: &[u8]) -> (Account, Self) {
        let mut instance = self.clone();
        instance.address = Account::try_from(blake2b(32, &[], salt).as_bytes()).unwrap();
        instance.storage.clear();
        (instance.address, instance)
    }

    fn instantiate(&self, vm: Runtime) -> Result<(Store<Runtime>, Instance), Error> {
        let engine = Engine::default();
        let mut store = Store::new(&engine, vm);

        let mut linker = <Linker<Runtime>>::new(&engine);
        Runtime::define(&mut store, &mut linker);
        let memory = Memory::new(&mut store, MemoryType::new(16, Some(16)).unwrap()).unwrap();
        linker.define("env", "memory", memory).unwrap();
        store.data_mut().memory = Some(memory);

        let instance = linker
            .instantiate(&mut store, &Module::new(&engine, &mut &self.code[..])?)?
            .ensure_no_start(&mut store)
            .expect("we never emit a start function");

        Ok((store, instance))
    }

    #[allow(clippy::result_large_err)] // eDONTCARE
    fn execute(&self, function: &str, vm: Runtime) -> Result<Store<Runtime>, (Error, String)> {
        let (mut store, instance) = self.instantiate(vm).map_err(|e| (e, String::new()))?;

        match instance
            .get_export(&store, function)
            .and_then(|export| export.into_func())
            .expect("contract does not export '{function}'")
            .call(&mut store, &[], &mut [])
        {
            Err(Error::Trap(trap)) if trap.trap_code().is_some() => {
                Err((Error::Trap(trap), store.data().debug_buffer.clone()))
            }
            Err(Error::Trap(trap)) => match trap.downcast::<HostReturn>() {
                Some(HostReturn::Data(flags, data)) => {
                    store.data_mut().output = HostReturn::Data(flags, data);
                    Ok(store)
                }
                Some(HostReturn::Terminate) => Ok(store),
                _ => panic!("contract execution stopped by unexpected trap"),
            },
            Err(e) => panic!("unexpected error during contract execution: {e}"),
            Ok(_) => Ok(store),
        }
    }
}

#[derive(Default, Clone)]
struct Runtime {
    contracts: Vec<Contract>,
    debug_buffer: String,
    events: Vec<Event>,
    caller: Account,
    contract: usize,
    memory: Option<Memory>,
    input: Vec<u8>,
    output: HostReturn,
    value: u128,
}

impl Runtime {
    fn new(contracts: Vec<Contract>) -> Self {
        Self {
            contracts,
            caller: rand::random(),
            ..Default::default()
        }
    }

    fn accept_state(&mut self, callee_state: Self, transferred_value: u128) {
        self.debug_buffer = callee_state.debug_buffer;
        self.events = callee_state.events;
        self.contracts = callee_state.contracts;
        self.contracts[self.contract].value -= transferred_value;
    }

    fn prepare_call_context(&self, callee: usize, input: Vec<u8>, value: u128) -> Self {
        let mut runtime = self.clone();
        runtime.caller = runtime.contracts[runtime.contract].address;
        runtime.contract = callee;
        runtime.value = value;
        runtime.input = input;
        runtime.output = Default::default();
        runtime.contracts[callee].value += value;
        runtime
    }

    fn call(
        &mut self,
        export: &str,
        callee: Account,
        input: Vec<u8>,
        value: u128,
    ) -> Result<Store<Runtime>, Error> {
        println!(
            "{export}: account={} input={} value={value}",
            hex::encode(callee),
            hex::encode(&input)
        );

        let (index, _) = self
            .contracts
            .iter()
            .enumerate()
            .find(|(_, contract)| contract.address == callee)
            .ok_or(Error::Trap(Trap::from(TrapCode::UnreachableCodeReached)))?; // TODO this should return 7 (CodeNotFound)

        self.contracts[index]
            .execute(export, self.prepare_call_context(index, input, value))
            .map_err(|(err, debug_buffer)| {
                self.debug_buffer = debug_buffer;
                err
            })
    }

    fn deploy(
        &mut self,
        code_hash: [u8; 32],
        value: u128,
        salt: &[u8],
        input: Vec<u8>,
    ) -> Result<Store<Runtime>, Error> {
        let (address, contract) = self
            .contracts
            .iter()
            .find(|contract| contract.code_hash == code_hash)
            .unwrap_or_else(|| panic!("code hash {} not found", hex::encode(code_hash)))
            .create_instance(salt);

        if self.contracts.iter().any(|c| c.address == address) {
            return Err(Error::Trap(TrapCode::UnreachableCodeReached.into()));
        }

        self.contracts.push(contract);
        self.call("deploy", address, input, value)
    }
}

fn read_len(mem: &[u8], ptr: u32) -> usize {
    u32::from_le_bytes(mem[ptr as usize..ptr as usize + 4].try_into().unwrap()) as usize
}

fn write_buf(mem: &mut [u8], ptr: u32, buf: &[u8]) {
    mem[ptr as usize..ptr as usize + buf.len()].copy_from_slice(buf);
}

fn read_buf(mem: &[u8], ptr: u32, len: u32) -> Vec<u8> {
    mem[ptr as usize..(ptr + len) as usize].to_vec()
}

fn read_value(mem: &[u8], ptr: u32) -> u128 {
    u128::from_le_bytes(read_buf(mem, ptr, 16).try_into().unwrap())
}

fn read_account(mem: &[u8], ptr: u32) -> Account {
    Account::try_from(&mem[ptr as usize..(ptr + 32) as usize]).unwrap()
}

#[wasm_host]
impl Runtime {
    #[seal(0)]
    fn seal_input(dest_ptr: u32, len_ptr: u32) -> Result<(), Trap> {
        assert!(read_len(mem, len_ptr) >= vm.input.len());
        println!("seal_input: {}", hex::encode(&vm.input));

        write_buf(mem, dest_ptr, &vm.input);
        write_buf(mem, len_ptr, &(vm.input.len() as u32).to_le_bytes());

        Ok(())
    }

    #[seal(0)]
    fn seal_return(flags: u32, data_ptr: u32, data_len: u32) -> Result<(), Trap> {
        let output = read_buf(mem, data_ptr, data_len);
        println!("seal_return: {flags} {}", hex::encode(&output));
        Err(HostReturn::Data(flags, output).into())
    }

    #[seal(0)]
    fn seal_value_transferred(dest_ptr: u32, out_len_ptr: u32) -> Result<(), Trap> {
        let value = vm.value.to_le_bytes();
        assert!(read_len(mem, out_len_ptr) >= value.len());
        println!("seal_value_transferred: {}", vm.value);

        write_buf(mem, dest_ptr, &value);
        write_buf(mem, out_len_ptr, &(value.len() as u32).to_le_bytes());

        Ok(())
    }

    #[seal(0)]
    fn seal_debug_message(data_ptr: u32, len: u32) -> Result<u32, Trap> {
        let buf = read_buf(mem, data_ptr, len);
        let msg = std::str::from_utf8(&buf).expect("seal_debug_message: Invalid UFT8");
        println!("seal_debug_message: {msg}");
        vm.debug_buffer.push_str(msg);
        Ok(0)
    }

    #[seal(1)]
    fn seal_get_storage(
        key_ptr: u32,
        key_len: u32,
        out_ptr: u32,
        out_len_ptr: u32,
    ) -> Result<u32, Trap> {
        let key = StorageKey::try_from(read_buf(mem, key_ptr, key_len))
            .expect("storage key size must be 32 bytes");
        println!(
            "get_storage value {:?}",
            vm.contracts[vm.contract].storage.get(&key)
        );
        let value = match vm.contracts[vm.contract].storage.get(&key) {
            Some(value) => value,
            _ => return Ok(3), // In pallet-contracts, ReturnCode::KeyNotFound == 3
        };
        println!("get_storage: {}={}", hex::encode(key), hex::encode(value));

        write_buf(mem, out_ptr, value);
        write_buf(mem, out_len_ptr, &(value.len() as u32).to_le_bytes());

        Ok(0)
    }

    #[seal(2)]
    fn seal_set_storage(
        key_ptr: u32,
        key_len: u32,
        value_ptr: u32,
        value_len: u32,
    ) -> Result<u32, Trap> {
        let key = StorageKey::try_from(read_buf(mem, key_ptr, key_len))
            .expect("storage key size must be 32 bytes");
        let value = mem[value_ptr as usize..(value_ptr + value_len) as usize].to_vec();
        println!("set_storage: {}={}", hex::encode(key), hex::encode(&value));

        match vm.contracts[vm.contract].storage.insert(key, value) {
            Some(value) => Ok(value.len() as u32),
            _ => Ok(u32::MAX), // In pallets contract, u32::MAX is the "none sentinel"
        }
    }

    #[seal(1)]
    fn seal_clear_storage(key_ptr: u32, key_len: u32) -> Result<u32, Trap> {
        let key = StorageKey::try_from(read_buf(mem, key_ptr, key_len))
            .expect("storage key size must be 32 bytes");
        println!("clear_storage: {}", hex::encode(key));

        match vm.contracts[vm.contract].storage.remove(&key) {
            Some(value) => Ok(value.len() as u32),
            _ => Ok(u32::MAX), // In pallets contract, u32::MAX is the "none sentinel"
        }
    }

    #[seal(0)]
    fn seal_hash_keccak_256(input_ptr: u32, input_len: u32, output_ptr: u32) -> Result<(), Trap> {
        let mut hasher = Keccak::v256();
        hasher.update(&read_buf(mem, input_ptr, input_len));
        hasher.finalize(&mut mem[output_ptr as usize..(output_ptr + 32) as usize]);
        Ok(())
    }

    #[seal(0)]
    fn seal_hash_sha2_256(input_ptr: u32, input_len: u32, output_ptr: u32) -> Result<(), Trap> {
        let mut hasher = Sha256::new();
        hasher.update(read_buf(mem, input_ptr, input_len));
        write_buf(mem, output_ptr, &hasher.finalize());
        Ok(())
    }

    #[seal(0)]
    fn seal_hash_blake2_128(input_ptr: u32, input_len: u32, output_ptr: u32) -> Result<(), Trap> {
        let data = read_buf(mem, input_ptr, input_len);
        write_buf(mem, output_ptr, blake2b(16, &[], &data).as_bytes());
        Ok(())
    }

    #[seal(0)]
    fn seal_hash_blake2_256(input_ptr: u32, input_len: u32, output_ptr: u32) -> Result<(), Trap> {
        let data = read_buf(mem, input_ptr, input_len);
        write_buf(mem, output_ptr, blake2b(32, &[], &data).as_bytes());
        Ok(())
    }

    #[seal(1)]
    fn seal_call(
        _flags: u32,
        callee_ptr: u32,
        _gas: u64,
        value_ptr: u32,
        input_ptr: u32,
        input_len: u32,
        output_ptr: u32,
        output_len_ptr: u32,
    ) -> Result<u32, Trap> {
        let callee = read_account(mem, callee_ptr);
        let input = read_buf(mem, input_ptr, input_len);
        let value = read_value(mem, value_ptr);

        if value > vm.contracts[vm.contract].value {
            return Ok(5); // ReturnCode::TransferFailed
        }

        let ((flags, data), state) = match vm.call("call", callee, input, value) {
            Ok(state) => ((state.data().output.as_data()), state),
            Err(_) => return Ok(1), // ReturnCode::CalleeTrapped
        };

        if output_len_ptr != u32::MAX {
            assert!(read_len(mem, output_len_ptr) >= data.len());
            write_buf(mem, output_ptr, &data);
            write_buf(mem, output_len_ptr, &(data.len() as u32).to_le_bytes());
        }

        if flags == 2 {
            return Ok(2); // ReturnCode::CalleeReverted
        }

        vm.accept_state(state.into_data(), value);
        Ok(0)
    }

    #[seal(0)]
    fn instantiation_nonce() -> Result<u64, Trap> {
        Ok(vm.contracts.len() as u64)
    }

    #[seal(0)]
    fn seal_minimum_balance(out_ptr: u32, out_len_ptr: u32) -> Result<(), Trap> {
        assert!(read_len(mem, out_len_ptr) >= 16);
        write_buf(mem, out_ptr, &500u128.to_le_bytes());
        Ok(())
    }

    #[seal(1)]
    fn seal_instantiate(
        code_hash_ptr: u32,
        _gas: u64,
        value_ptr: u32,
        input_data_ptr: u32,
        input_data_len: u32,
        address_ptr: u32,
        address_len_ptr: u32,
        output_ptr: u32,
        output_len_ptr: u32,
        salt_ptr: u32,
        salt_len: u32,
    ) -> Result<u32, Trap> {
        let target = read_account(mem, code_hash_ptr);
        let salt = read_buf(mem, salt_ptr, salt_len);
        let input = read_buf(mem, input_data_ptr, input_data_len);
        let value = read_value(mem, value_ptr);

        if value > vm.contracts[vm.contract].value {
            return Ok(5); // ReturnCode::TransferFailed
        }

        let ((flags, data), state) = match vm.deploy(target, value, &salt, input) {
            Ok(state) => ((state.data().output.as_data()), state),
            Err(_) => return Ok(1), // ReturnCode::CalleeTrapped
        };

        if output_len_ptr != u32::MAX {
            write_buf(mem, output_ptr, &data);
            let output_len = &(data.len() as u32).to_le_bytes();
            write_buf(mem, output_len_ptr, output_len);

            let address = state.data().contracts.last().unwrap().address;
            write_buf(mem, address_ptr, &address);
            write_buf(mem, address_len_ptr, &(address.len() as u32).to_le_bytes());
        }

        if flags == 2 {
            return Ok(2); // ReturnCode::CalleeReverted
        }

        vm.accept_state(state.into_data(), value);
        Ok(0)
    }

    #[seal(0)]
    fn seal_transfer(account_ptr: u32, _: u32, value_ptr: u32, _: u32) -> Result<u32, Trap> {
        let value = read_value(mem, value_ptr);
        if value > vm.contracts[vm.contract].value {
            return Ok(5); // ReturnCode::TransferFailed
        }

        let accout = read_account(mem, account_ptr);
        if let Some(to) = vm.contracts.iter_mut().find(|c| c.address == accout) {
            to.value += value;
            vm.contracts[vm.contract].value -= value;
            return Ok(0);
        }

        Ok(5)
    }

    #[seal(0)]
    fn seal_address(out_ptr: u32, out_len_ptr: u32) -> Result<(), Trap> {
        let address = vm.contracts[vm.contract].address;
        let out_len = read_len(mem, out_len_ptr);
        assert!(out_len >= address.len());

        write_buf(mem, out_ptr, &address);
        write_buf(mem, out_len_ptr, &(address.len() as u32).to_le_bytes());

        Ok(())
    }

    #[seal(0)]
    fn seal_caller(out_ptr: u32, out_len_ptr: u32) -> Result<(), Trap> {
        let out_len = read_len(mem, out_len_ptr);
        assert!(out_len >= vm.caller.len());

        write_buf(mem, out_ptr, &vm.caller);
        write_buf(mem, out_len_ptr, &(vm.caller.len() as u32).to_le_bytes());

        Ok(())
    }

    #[seal(0)]
    fn seal_balance(out_ptr: u32, out_len_ptr: u32) -> Result<(), Trap> {
        let balance = vm.contracts[vm.contract].value.to_le_bytes();
        let out_len = read_len(mem, out_len_ptr);
        assert!(out_len >= balance.len());

        write_buf(mem, out_ptr, &balance);
        write_buf(mem, out_len_ptr, &(balance.len() as u32).to_le_bytes());

        Ok(())
    }

    #[seal(0)]
    fn seal_block_number(out_ptr: u32, out_len_ptr: u32) -> Result<(), Trap> {
        let block = 950_119_597u32.to_le_bytes();
        let out_len = read_len(mem, out_len_ptr);
        assert!(out_len >= block.len());

        write_buf(mem, out_ptr, &block);
        write_buf(mem, out_len_ptr, &(block.len() as u32).to_le_bytes());

        Ok(())
    }

    #[seal(0)]
    fn seal_now(out_ptr: u32, out_len_ptr: u32) -> Result<(), Trap> {
        let now = 1594035638000u64.to_le_bytes();
        let out_len = read_len(mem, out_len_ptr);
        assert!(out_len >= now.len());

        write_buf(mem, out_ptr, &now);
        write_buf(mem, out_len_ptr, &(now.len() as u32).to_le_bytes());

        Ok(())
    }

    #[seal(0)]
    fn seal_gas_left(out_ptr: u32, out_len_ptr: u32) -> Result<(), Trap> {
        let gas = 2_224_097_461u64.to_le_bytes();
        let out_len = read_len(mem, out_len_ptr);
        assert!(out_len >= gas.len());

        write_buf(mem, out_ptr, &gas);
        write_buf(mem, out_len_ptr, &(gas.len() as u32).to_le_bytes());

        Ok(())
    }

    #[seal(0)]
    fn seal_weight_to_fee(gas: u64, out_ptr: u32, out_len_ptr: u32) -> Result<(), Trap> {
        let price = (59_541_253_813_967 * gas as u128).to_le_bytes();
        let out_len = read_len(mem, out_len_ptr);
        assert!(out_len >= price.len());

        write_buf(mem, out_ptr, &price);
        write_buf(mem, out_len_ptr, &(price.len() as u32).to_le_bytes());

        Ok(())
    }

    #[seal(1)]
    fn seal_terminate(beneficiary_ptr: u32) -> Result<(), Trap> {
        let free = vm.contracts.remove(vm.contract).value;
        let address = read_account(mem, beneficiary_ptr);
        println!("seal_terminate: {} gets {free}", hex::encode(address));

        if let Some(to) = vm.contracts.iter_mut().find(|c| c.address == address) {
            to.value += free;
        }

        Err(HostReturn::Terminate.into())
    }

    #[seal(0)]
    fn seal_deposit_event(
        topics_ptr: u32,
        topics_len: u32,
        data_ptr: u32,
        data_len: u32,
    ) -> Result<(), Trap> {
        let data = read_buf(mem, data_ptr, data_len);
        let topics = if topics_len > 0 {
            <Vec<Hash>>::decode(&mut &read_buf(mem, topics_ptr, topics_len)[..]).unwrap()
        } else {
            vec![]
        };

        println!("seal_deposit_event data: {}", hex::encode(&data));
        println!("seal_deposit_event topics: {topics:?}");
        vm.events.push(Event { topics, data });

        Ok(())
    }
}

pub struct MockSubstrate {
    state: Store<Runtime>,
    pub current_program: usize,
    pub account: Account,
    pub value: u128,
}

#[derive(Default, Debug, Clone)]
enum HostReturn {
    #[default]
    Terminate,
    Data(u32, Vec<u8>),
}

impl HostReturn {
    fn as_data(&self) -> (u32, Vec<u8>) {
        match self {
            HostReturn::Data(flags, data) => (*flags, data.to_vec()),
            HostReturn::Terminate => (0, vec![]),
        }
    }
}

impl fmt::Display for HostReturn {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            Self::Terminate => write!(f, "return: terminate"),
            Self::Data(flags, data) => write!(f, "return {flags} {data:?}"),
        }
    }
}

impl HostError for HostReturn {}

#[derive(Clone, Debug)]
pub struct Event {
    topics: Vec<Hash>,
    data: Vec<u8>,
}

impl MockSubstrate {
    fn invoke(&mut self, export: &str, input: Vec<u8>, value: u128) -> Result<(), Error> {
        let callee = self.state.data().contracts[self.current_program].address;
        self.account = callee;
        self.state.data_mut().debug_buffer.clear();
        self.state.data_mut().events.clear();
        self.state = self.state.data_mut().call(export, callee, input, value)?;
        Ok(())
    }

    pub fn set_program(&mut self, index: usize) {
        self.current_program = index;
    }

    pub fn output(&self) -> Vec<u8> {
        if let HostReturn::Data(_, data) = &self.state.data().output {
            return data.to_vec();
        }
        vec![]
    }

    pub fn caller(&self) -> Account {
        self.state.data().caller
    }

    pub fn debug_buffer(&self) -> String {
        self.state.data().debug_buffer.clone()
    }

    pub fn events(&self) -> Vec<Event> {
        self.state.data().events.clone()
    }

    pub fn contracts(&self) -> &[Contract] {
        &self.state.data().contracts
    }

    pub fn storage(&self) -> &HashMap<[u8; 32], Vec<u8>> {
        &self.state.data().contracts[self.current_program].storage
    }

    pub fn value(&self, contract: usize) -> u128 {
        self.state.data().contracts[contract].value
    }

    pub fn selector(&self, contract: usize, function_name: &str) -> &[u8] {
        &self.state.data().contracts[contract].messages[function_name]
    }

    pub fn constructor(&mut self, index: usize, mut args: Vec<u8>) {
        let mut input =
            self.state.data().contracts[self.current_program].constructors[index].clone();
        input.append(&mut args);
        self.raw_constructor(input);
    }

    pub fn raw_constructor(&mut self, input: Vec<u8>) {
        self.invoke("deploy", input, 20000).unwrap();
    }

    pub fn function(&mut self, name: &str, mut args: Vec<u8>) {
        let mut input = self.state.data().contracts[self.current_program].messages[name].clone();
        input.append(&mut args);
        self.raw_function(input, self.value);
    }

    pub fn function_expect_failure(&mut self, name: &str, mut args: Vec<u8>) {
        let mut input = self.state.data().contracts[self.current_program].messages[name].clone();
        input.append(&mut args);
        self.raw_function_failure(input, self.value);
    }

    pub fn raw_function(&mut self, input: Vec<u8>, value: u128) {
        self.invoke("call", input, value).unwrap();
    }

    pub fn raw_function_failure(&mut self, input: Vec<u8>, value: u128) {
        match self.invoke("call", input, value) {
            Err(wasmi::Error::Trap(trap)) => match trap.trap_code() {
                Some(TrapCode::UnreachableCodeReached) => (),
                _ => panic!("trap: {trap:?}"),
            },
            Err(err) => panic!("unexpected error: {err:?}"),
            Ok(_) => panic!("unexpected return from main"),
        }
    }

    pub fn heap_verify(&mut self) {
        let mem = self.state.data().memory.unwrap().data(&mut self.state);
        let memsize = mem.len();
        println!("memory size:{memsize}");
        let mut buf = Vec::new();
        buf.resize(memsize, 0);

        let mut current_elem = 0x10000;
        let mut last_elem = 0u32;

        let read_u32 = |ptr| u32::from_le_bytes(mem[ptr..ptr + 4].try_into().unwrap());

        loop {
            let next: u32 = read_u32(current_elem);
            let prev: u32 = read_u32(current_elem + 4);
            let length: u32 = read_u32(current_elem + 8);
            let allocated: u32 = read_u32(current_elem + 12);

            println!("next:{next:08x} prev:{prev:08x} length:{length} allocated:{allocated}");

            let buf = read_buf(mem, current_elem as u32 + 16, length);

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
                        write!(hex, " {b:02x}").unwrap();
                        if b.is_ascii() && !b.is_ascii_control() {
                            write!(chars, "  {}", b as char).unwrap();
                        } else {
                            chars.push_str("   ");
                        }
                    }
                    println!("{hex}\n{chars}");
                }
            }

            assert_eq!(last_elem, prev);

            if next == 0 {
                break;
            }

            last_elem = current_elem as u32;
            current_elem = next as usize;
        }
    }
}

pub fn build_solidity(src: &str) -> MockSubstrate {
    build_solidity_with_options(src, false, true)
}

pub fn build_solidity_with_options(src: &str, log_ret: bool, log_err: bool) -> MockSubstrate {
    let contracts = build_wasm(src, log_ret, log_err)
        .iter()
        .map(|(code, abi)| Contract::new(abi, code))
        .collect();

    MockSubstrate {
        state: Store::new(&Engine::default(), Runtime::new(contracts)),
        account: Default::default(),
        current_program: 0,
        value: 0,
    }
}

fn build_wasm(src: &str, log_ret: bool, log_err: bool) -> Vec<(Vec<u8>, String)> {
    let tmp_file = OsStr::new("test.sol");
    let mut cache = FileResolver::new();
    cache.set_file_contents(tmp_file.to_str().unwrap(), src.to_string());
    let opt = inkwell::OptimizationLevel::Default;
    let target = Target::default_substrate();
    let (wasm, ns) = compile(tmp_file, &mut cache, opt, target, log_ret, log_err, true);
    ns.print_diagnostics_in_plain(&cache, false);
    assert!(!wasm.is_empty());
    wasm
}

fn load_abi(s: &str) -> InkProject {
    let bundle = serde_json::from_str::<ContractMetadata>(s).unwrap();
    serde_json::from_value::<InkProject>(serde_json::to_value(bundle.abi).unwrap()).unwrap()
}
