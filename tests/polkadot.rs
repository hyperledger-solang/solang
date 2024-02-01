// SPDX-License-Identifier: Apache-2.0

/// Mock runtime for the contracts pallet.
use blake2_rfc::blake2b::blake2b;
use contract_metadata::ContractMetadata;
use ink_metadata::InkProject;
use ink_primitives::Hash;
use parity_scale_codec::Decode;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::{collections::HashMap, ffi::OsStr, fmt, fmt::Write};
use tiny_keccak::{Hasher, Keccak};
use wasmi::core::{HostError, Trap, TrapCode};
use wasmi::{Engine, Error, Instance, Linker, Memory, MemoryType, Module, Store};

use solang::codegen::Options;
use solang::file_resolver::FileResolver;
use solang::{compile, Target};

use wasm_host_attr::wasm_host;

mod polkadot_tests;

type StorageKey = [u8; 32];
type Address = [u8; 32];

#[derive(Clone, Copy)]
enum CallFlags {
    ForwardInput = 1,
    CloneInput = 2,
    TailCall = 4,
    AllowReentry = 8,
}

impl CallFlags {
    /// Returns true if this flag is set in the given `flags`.
    fn set(&self, flags: u32) -> bool {
        flags & *self as u32 != 0
    }
}

/// Reason for halting execution. Same as in pallet contracts.
#[derive(Default, Debug, Clone)]
enum HostReturn {
    /// The contract was terminated (deleted).
    #[default]
    Terminate,
    /// Flags and data returned by the contract.
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

/// Represents a contract code artifact.
#[derive(Clone)]
pub struct WasmCode {
    /// A mapping from function names to selectors.
    messages: HashMap<String, Vec<u8>>,
    /// A list of the selectors of the constructors.
    constructors: Vec<Vec<u8>>,
    hash: Hash,
    blob: Vec<u8>,
}

impl WasmCode {
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
            .collect();

        Self {
            messages,
            constructors,
            hash: blake2b(32, &[], code).as_bytes().try_into().unwrap(),
            blob: code.to_vec(),
        }
    }
}

/// A `Contract` represent deployed Wasm code with its storage which can be executed.
#[derive(Clone)]
pub struct Contract {
    code: WasmCode,
    storage: HashMap<StorageKey, Vec<u8>>,
}

impl From<WasmCode> for Contract {
    fn from(code: WasmCode) -> Self {
        Self {
            code,
            storage: HashMap::new(),
        }
    }
}

impl Contract {
    /// Instantiate this contract as a Wasm module for execution.
    fn instantiate(&self, runtime: Runtime) -> Result<(Store<Runtime>, Instance), Error> {
        let engine = Engine::default();
        let mut store = Store::new(&engine, runtime);

        let mut linker = <Linker<Runtime>>::new(&engine);
        Runtime::define(&mut store, &mut linker);
        let memory = Memory::new(&mut store, MemoryType::new(16, Some(16)).unwrap()).unwrap();
        linker.define("env", "memory", memory).unwrap();
        store.data_mut().memory = Some(memory);

        let instance = linker
            .instantiate(&mut store, &Module::new(&engine, &mut &self.code.blob[..])?)?
            .ensure_no_start(&mut store)
            .expect("we never emit a start function");

        Ok((store, instance))
    }

    /// Execute this contract at the exportet function `name` in the given `runtime` context.
    ///
    /// On success, returns the Wasm store including the runtime state is returned.
    /// On failure, returns the Wasm execution Error together with the debug buffer.
    #[allow(clippy::result_large_err)] // eDONTCARE
    fn execute(&self, name: &str, runtime: Runtime) -> Result<Store<Runtime>, (Error, String)> {
        let (mut store, instance) = self.instantiate(runtime).map_err(|e| (e, String::new()))?;

        match instance
            .get_export(&store, name)
            .and_then(|export| export.into_func())
            .unwrap_or_else(|| panic!("contract does not export '{name}'"))
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

/// If contract is `Some`, this is considered to be a "contract account".
#[derive(Default, Clone)]
struct Account {
    address: Address,
    value: u128,
    contract: Option<Contract>,
}

impl PartialEq for Account {
    fn eq(&self, other: &Self) -> bool {
        self.address == other.address
    }
}

impl Account {
    /// Create a new contract account.
    /// The account address is derived based of the provided `salt`.
    fn with_contract(salt: &[u8], code: &WasmCode) -> Self {
        Self {
            address: Address::try_from(blake2b(32, &[], salt).as_bytes()).unwrap(),
            contract: Some(code.clone().into()),
            ..Default::default()
        }
    }
}

#[derive(Clone)]
pub struct Event {
    pub data: Vec<u8>,
    pub topics: Vec<Hash>,
}

/// The runtime provides the state of the mocked blockchain node during contract execution.
#[derive(Default, Clone)]
struct Runtime {
    /// A list of "existing" accounts.
    accounts: Vec<Account>,
    /// A list of known ("uploaded") Wasm contract blobs.
    blobs: Vec<WasmCode>,
    /// Index into accounts pointing the account that is being executed.
    account: usize,
    /// Index into accounts pointing to the calling account.
    caller_account: usize,
    /// Will hold the memory reference after a successful execution.
    memory: Option<Memory>,
    /// The input for the contract execution.
    input: Option<Vec<u8>>,
    /// The output of the contract execution.
    output: HostReturn,
    /// Descirbes how much value was given to the contract call.
    transferred_value: u128,
    /// Combined ouptut of all `seal_debug_message` calls
    debug_buffer: String,
    /// Stores all events emitted during contract execution.
    events: Vec<Event>,
    /// The set of called events, needed for reentrancy protection.
    called_accounts: HashSet<usize>,
}

impl Runtime {
    fn new(blobs: Vec<WasmCode>) -> Self {
        Self {
            accounts: blobs
                .iter()
                .map(|blob| Account::with_contract(blob.hash.as_ref(), blob))
                .collect(),
            blobs,
            ..Default::default()
        }
    }

    /// Create a suitable runtime context based on the current one.
    ///
    /// Each contract execution must live within it's own runtime context.
    /// When calling into another contract, we must:
    /// * switch out the caller and callee account
    /// * populate the input and the transferred balance
    /// * clear the output
    fn new_context(&self, callee: usize, input: Vec<u8>, value: u128) -> Self {
        let mut runtime = self.clone();
        runtime.caller_account = self.account;
        runtime.account = callee;
        runtime.transferred_value = value;
        runtime.accounts[callee].value += value;
        runtime.input = Some(input);
        runtime.output = Default::default();
        runtime.called_accounts.insert(self.caller_account);
        runtime
    }

    /// After a succesfull contract execution, merge the runtime context of the callee back.
    ///
    /// We take over accounts (the callee might deploy new ones), debug buffer and emitted events.
    /// The transferred balance will now be deducted from the caller.
    fn accept_state(&mut self, callee_state: Self, transferred_value: u128) {
        self.debug_buffer = callee_state.debug_buffer;
        self.events = callee_state.events;
        self.accounts = callee_state.accounts;
        self.accounts[self.caller_account].value -= transferred_value;
    }

    /// Access the contract that is currently being executed.
    fn contract(&mut self) -> &mut Contract {
        self.accounts[self.account].contract.as_mut().unwrap()
    }

    /// Call an exported function under the account found at index `callee`.
    ///
    /// Returns `None` if the account has no contract.
    fn call(
        &mut self,
        export: &str,
        callee: usize,
        input: Vec<u8>,
        value: u128,
    ) -> Option<Result<Store<Runtime>, Error>> {
        println!(
            "{export}: account={} input={} value={value}",
            hex::encode(self.accounts[callee].address),
            hex::encode(&input)
        );

        self.accounts[callee]
            .contract
            .as_ref()?
            .execute(export, self.new_context(callee, input, value))
            .map_err(|(err, debug_buffer)| {
                self.debug_buffer = debug_buffer;
                err
            })
            .into()
    }

    /// Add a new contract account and call its "deploy" function accordingly.
    ///
    /// Returns `None` if there is no contract corresponding to the given `code_hash`.
    fn deploy(
        &mut self,
        code_hash: Hash,
        value: u128,
        salt: &[u8],
        input: Vec<u8>,
    ) -> Option<Result<Store<Runtime>, Error>> {
        let account = self
            .blobs
            .iter()
            .find(|code| code.hash == code_hash)
            .map(|code| Account::with_contract(salt, code))?;

        if self.accounts.contains(&account) {
            return Some(Err(Error::Trap(TrapCode::UnreachableCodeReached.into())));
        }

        self.accounts.push(account);
        self.call("deploy", self.accounts.len() - 1, input, value)
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

fn read_account(mem: &[u8], ptr: u32) -> Address {
    Address::try_from(&mem[ptr as usize..(ptr + 32) as usize]).unwrap()
}

fn read_hash(mem: &[u8], ptr: u32) -> Hash {
    Hash::try_from(&mem[ptr as usize..(ptr + 32) as usize]).unwrap()
}

/// Host functions mock the original implementation, refer to the [pallet docs][1] for more information.
///
/// [1]: https://docs.rs/pallet-contracts/latest/pallet_contracts/api_doc/index.html
///
/// Address `[0; u8]` is considered the root account.
#[wasm_host]
impl Runtime {
    #[seal(0)]
    fn input(dest_ptr: u32, len_ptr: u32) -> Result<(), Trap> {
        let data = vm.input.as_ref().expect("input was forwarded");
        assert!(read_len(mem, len_ptr) >= data.len());
        println!("seal_input: {}", hex::encode(data));

        write_buf(mem, dest_ptr, data);
        write_buf(mem, len_ptr, &(data.len() as u32).to_le_bytes());

        Ok(())
    }

    #[seal(0)]
    fn seal_return(flags: u32, data_ptr: u32, data_len: u32) -> Result<(), Trap> {
        let output = read_buf(mem, data_ptr, data_len);
        println!("seal_return: {flags} {}", hex::encode(&output));
        Err(HostReturn::Data(flags, output).into())
    }

    #[seal(0)]
    fn value_transferred(dest_ptr: u32, out_len_ptr: u32) -> Result<(), Trap> {
        let value = vm.transferred_value.to_le_bytes();
        assert!(read_len(mem, out_len_ptr) >= value.len());
        println!("seal_value_transferred: {}", vm.transferred_value);

        write_buf(mem, dest_ptr, &value);
        write_buf(mem, out_len_ptr, &(value.len() as u32).to_le_bytes());

        Ok(())
    }

    #[seal(0)]
    fn debug_message(data_ptr: u32, len: u32) -> Result<u32, Trap> {
        let buf = read_buf(mem, data_ptr, len);
        let msg = std::str::from_utf8(&buf).expect("seal_debug_message: Invalid UFT8");
        println!("seal_debug_message: {msg}");
        vm.debug_buffer.push_str(msg);
        Ok(0)
    }

    #[seal(1)]
    fn get_storage(
        key_ptr: u32,
        key_len: u32,
        out_ptr: u32,
        out_len_ptr: u32,
    ) -> Result<u32, Trap> {
        let key = StorageKey::try_from(read_buf(mem, key_ptr, key_len))
            .expect("storage key size must be 32 bytes");
        let value = match vm.contract().storage.get(&key) {
            Some(value) => value,
            _ => return Ok(3), // In pallet-contracts, ReturnCode::KeyNotFound == 3
        };
        println!("get_storage: {}={}", hex::encode(key), hex::encode(value));

        write_buf(mem, out_ptr, value);
        write_buf(mem, out_len_ptr, &(value.len() as u32).to_le_bytes());

        Ok(0)
    }

    #[seal(2)]
    fn set_storage(
        key_ptr: u32,
        key_len: u32,
        value_ptr: u32,
        value_len: u32,
    ) -> Result<u32, Trap> {
        let key = StorageKey::try_from(read_buf(mem, key_ptr, key_len))
            .expect("storage key size must be 32 bytes");
        let value = mem[value_ptr as usize..(value_ptr + value_len) as usize].to_vec();
        println!("set_storage: {}={}", hex::encode(key), hex::encode(&value));

        match vm.contract().storage.insert(key, value) {
            Some(value) => Ok(value.len() as u32),
            _ => Ok(u32::MAX), // In pallets contract, u32::MAX is the "none sentinel"
        }
    }

    #[seal(1)]
    fn clear_storage(key_ptr: u32, key_len: u32) -> Result<u32, Trap> {
        let key = StorageKey::try_from(read_buf(mem, key_ptr, key_len))
            .expect("storage key size must be 32 bytes");
        println!("clear_storage: {}", hex::encode(key));

        match vm.contract().storage.remove(&key) {
            Some(value) => Ok(value.len() as u32),
            _ => Ok(u32::MAX), // In pallets contract, u32::MAX is the "none sentinel"
        }
    }

    #[seal(0)]
    fn hash_keccak_256(input_ptr: u32, input_len: u32, output_ptr: u32) -> Result<(), Trap> {
        let mut hasher = Keccak::v256();
        hasher.update(&read_buf(mem, input_ptr, input_len));
        hasher.finalize(&mut mem[output_ptr as usize..(output_ptr + 32) as usize]);
        Ok(())
    }

    #[seal(0)]
    fn hash_sha2_256(input_ptr: u32, input_len: u32, output_ptr: u32) -> Result<(), Trap> {
        let mut hasher = Sha256::new();
        hasher.update(read_buf(mem, input_ptr, input_len));
        write_buf(mem, output_ptr, &hasher.finalize());
        Ok(())
    }

    #[seal(0)]
    fn hash_blake2_128(input_ptr: u32, input_len: u32, output_ptr: u32) -> Result<(), Trap> {
        let data = read_buf(mem, input_ptr, input_len);
        write_buf(mem, output_ptr, blake2b(16, &[], &data).as_bytes());
        Ok(())
    }

    #[seal(0)]
    fn hash_blake2_256(input_ptr: u32, input_len: u32, output_ptr: u32) -> Result<(), Trap> {
        let data = read_buf(mem, input_ptr, input_len);
        write_buf(mem, output_ptr, blake2b(32, &[], &data).as_bytes());
        Ok(())
    }

    #[seal(1)]
    fn seal_call(
        flags: u32,
        callee_ptr: u32,
        _gas: u64,
        value_ptr: u32,
        input_ptr: u32,
        input_len: u32,
        output_ptr: u32,
        output_len_ptr: u32,
    ) -> Result<u32, Trap> {
        assert!(flags <= 0b1111);

        let input = if CallFlags::ForwardInput.set(flags) {
            if vm.input.is_none() {
                return Ok(1);
            }
            vm.input.take().unwrap()
        } else if CallFlags::CloneInput.set(flags) {
            if vm.input.is_none() {
                return Ok(1);
            }
            vm.input.as_ref().unwrap().clone()
        } else {
            read_buf(mem, input_ptr, input_len)
        };
        let value = read_value(mem, value_ptr);
        let callee_address = read_account(mem, callee_ptr);

        let callee = match vm
            .accounts
            .iter()
            .enumerate()
            .find(|(_, account)| account.address == callee_address)
            .map(|(index, _)| index)
        {
            Some(index) => index,
            None => return Ok(8), // ReturnCode::NotCallable
        };

        if vm.called_accounts.contains(&callee) && !CallFlags::AllowReentry.set(flags) {
            return Ok(1);
        }

        if value > vm.accounts[vm.account].value {
            return Ok(5); // ReturnCode::TransferFailed
        }

        let ((ret, data), state) = match vm.call("call", callee, input, value) {
            Some(Ok(state)) => ((state.data().output.as_data()), state),
            Some(Err(_)) => return Ok(1), // ReturnCode::CalleeTrapped
            None => return Ok(8),
        };

        if CallFlags::TailCall.set(flags) {
            return Err(HostReturn::Data(ret, data).into());
        }

        if output_len_ptr != u32::MAX {
            assert!(read_len(mem, output_len_ptr) >= data.len());
            write_buf(mem, output_ptr, &data);
            write_buf(mem, output_len_ptr, &(data.len() as u32).to_le_bytes());
        }

        if ret == 0 {
            vm.accept_state(state.into_data(), value);
            return Ok(0);
        }
        Ok(2) // Callee reverted
    }

    #[seal(0)]
    fn instantiation_nonce() -> Result<u64, Trap> {
        Ok(vm.accounts.len() as u64)
    }

    #[seal(0)]
    fn minimum_balance(out_ptr: u32, out_len_ptr: u32) -> Result<(), Trap> {
        assert!(read_len(mem, out_len_ptr) >= 16);
        write_buf(mem, out_ptr, &500u128.to_le_bytes());
        Ok(())
    }

    #[seal(1)]
    fn instantiate(
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
        let code_hash = read_hash(mem, code_hash_ptr);
        let salt = read_buf(mem, salt_ptr, salt_len);
        let input = read_buf(mem, input_data_ptr, input_data_len);
        let value = read_value(mem, value_ptr);

        if value > vm.accounts[vm.account].value {
            return Ok(5); // ReturnCode::TransferFailed
        }

        let ((flags, data), state) = match vm.deploy(code_hash, value, &salt, input) {
            Some(Ok(state)) => ((state.data().output.as_data()), state),
            Some(Err(_)) => return Ok(1), // ReturnCode::CalleeTrapped
            None => return Ok(7),         // ReturnCode::CodeNotFound
        };

        if output_len_ptr != u32::MAX {
            write_buf(mem, output_ptr, &data);
            write_buf(mem, output_len_ptr, &(data.len() as u32).to_le_bytes());
        }

        let address = state.data().accounts.last().unwrap().address;
        write_buf(mem, address_ptr, &address);
        write_buf(mem, address_len_ptr, &(address.len() as u32).to_le_bytes());

        if flags == 0 {
            vm.accept_state(state.into_data(), value);
            return Ok(0);
        }
        Ok(2) // Callee reverted
    }

    #[seal(0)]
    fn transfer(
        account_ptr: u32,
        account_len: u32,
        value_ptr: u32,
        value_len: u32,
    ) -> Result<u32, Trap> {
        assert_eq!(account_len, 32);
        assert_eq!(value_len, 16);

        let value = read_value(mem, value_ptr);
        if value > vm.accounts[vm.account].value {
            return Ok(5); // ReturnCode::TransferFailed
        }

        let account = read_account(mem, account_ptr);
        if let Some(to) = vm.accounts.iter_mut().find(|c| c.address == account) {
            to.value += value;
            vm.accounts[vm.account].value -= value;
            return Ok(0);
        }

        Ok(5)
    }

    #[seal(0)]
    fn address(out_ptr: u32, out_len_ptr: u32) -> Result<(), Trap> {
        let address = vm.accounts[vm.account].address;
        let out_len = read_len(mem, out_len_ptr);
        assert!(out_len >= address.len());

        write_buf(mem, out_ptr, &address);
        write_buf(mem, out_len_ptr, &(address.len() as u32).to_le_bytes());

        Ok(())
    }

    #[seal(0)]
    fn caller(out_ptr: u32, out_len_ptr: u32) -> Result<(), Trap> {
        let out_len = read_len(mem, out_len_ptr);
        let address = vm.accounts[vm.caller_account].address;
        assert!(out_len >= address.len());

        write_buf(mem, out_ptr, &address);
        write_buf(mem, out_len_ptr, &(address.len() as u32).to_le_bytes());

        Ok(())
    }

    #[seal(0)]
    fn balance(out_ptr: u32, out_len_ptr: u32) -> Result<(), Trap> {
        let balance = vm.accounts[vm.account].value.to_le_bytes();
        let out_len = read_len(mem, out_len_ptr);
        assert!(out_len >= balance.len());

        write_buf(mem, out_ptr, &balance);
        write_buf(mem, out_len_ptr, &(balance.len() as u32).to_le_bytes());

        Ok(())
    }

    #[seal(0)]
    fn block_number(out_ptr: u32, out_len_ptr: u32) -> Result<(), Trap> {
        let block = 950_119_597u32.to_le_bytes();
        let out_len = read_len(mem, out_len_ptr);
        assert!(out_len >= block.len());

        write_buf(mem, out_ptr, &block);
        write_buf(mem, out_len_ptr, &(block.len() as u32).to_le_bytes());

        Ok(())
    }

    #[seal(0)]
    fn now(out_ptr: u32, out_len_ptr: u32) -> Result<(), Trap> {
        let now = 1594035638000u64.to_le_bytes();
        let out_len = read_len(mem, out_len_ptr);
        assert!(out_len >= now.len());

        write_buf(mem, out_ptr, &now);
        write_buf(mem, out_len_ptr, &(now.len() as u32).to_le_bytes());

        Ok(())
    }

    #[seal(0)]
    fn gas_left(out_ptr: u32, out_len_ptr: u32) -> Result<(), Trap> {
        let gas = 2_224_097_461u64.to_le_bytes();
        let out_len = read_len(mem, out_len_ptr);
        assert!(out_len >= gas.len());

        write_buf(mem, out_ptr, &gas);
        write_buf(mem, out_len_ptr, &(gas.len() as u32).to_le_bytes());

        Ok(())
    }

    #[seal(0)]
    fn weight_to_fee(gas: u64, out_ptr: u32, out_len_ptr: u32) -> Result<(), Trap> {
        let price = (59_541_253_813_967 * gas as u128).to_le_bytes();
        let out_len = read_len(mem, out_len_ptr);
        assert!(out_len >= price.len());

        write_buf(mem, out_ptr, &price);
        write_buf(mem, out_len_ptr, &(price.len() as u32).to_le_bytes());

        Ok(())
    }

    #[seal(1)]
    fn terminate(beneficiary_ptr: u32) -> Result<(), Trap> {
        let free = vm.accounts.remove(vm.account).value;
        let address = read_account(mem, beneficiary_ptr);
        println!("seal_terminate: {} gets {free}", hex::encode(address));

        if let Some(to) = vm.accounts.iter_mut().find(|a| a.address == address) {
            to.value += free;
        }

        Err(HostReturn::Terminate.into())
    }

    #[seal(0)]
    fn deposit_event(
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

        println!(
            "seal_deposit_event data: {} topics: {:?}",
            hex::encode(&data),
            topics.iter().map(hex::encode).collect::<Vec<_>>()
        );

        vm.events.push(Event { data, topics });

        Ok(())
    }

    /// Mock chain extension with ID 123 that writes the reversed input to the output buf.
    /// Returns the sum of the input data.
    #[seal(0)]
    fn call_chain_extension(
        id: u32,
        input_ptr: u32,
        input_len: u32,
        output_ptr: u32,
        output_len_ptr: u32,
    ) -> Result<u32, Trap> {
        assert_eq!(id, 123, "unkown chain extension");
        assert!(read_len(mem, output_len_ptr) == 16384 && input_len <= 16384);

        let mut data = read_buf(mem, input_ptr, input_len);
        data.reverse();

        write_buf(mem, output_ptr, &data);
        write_buf(mem, output_len_ptr, &(data.len() as u32).to_le_bytes());

        Ok(data.iter().map(|i| *i as u32).sum())
    }

    #[seal(0)]
    fn is_contract(input_ptr: u32) -> Result<u32, Trap> {
        let address = read_account(mem, input_ptr);
        Ok(vm
            .accounts
            .iter()
            .any(|account| account.contract.is_some() && account.address == address)
            .into())
    }

    #[seal(0)]
    fn caller_is_root() -> Result<u32, Trap> {
        Ok((vm.accounts[vm.caller_account].address == [0; 32]).into())
    }

    #[seal(0)]
    fn set_code_hash(code_hash_ptr: u32) -> Result<u32, Trap> {
        let hash = read_hash(mem, code_hash_ptr);
        if let Some(code) = vm.blobs.iter().find(|code| code.hash == hash) {
            vm.accounts[vm.account].contract.as_mut().unwrap().code = code.clone();
            return Ok(0);
        }
        Ok(7) // ReturnCode::CodeNoteFound
    }
}

/// Provides a mock implementation of substrates [contracts pallet][1]
///
/// [1]: https://docs.rs/pallet-contracts/latest/pallet_contracts/index.html
pub struct MockSubstrate(Store<Runtime>);

impl MockSubstrate {
    fn invoke(&mut self, export: &str, input: Vec<u8>) -> Result<(), Error> {
        let callee = self.0.data().account;
        let value = self.0.data().transferred_value;
        let runtime = self.0.data_mut();

        runtime.debug_buffer.clear();
        runtime.events.clear();
        runtime.called_accounts.clear();
        self.0 = runtime.call(export, callee, input, value).unwrap()?;
        self.0.data_mut().transferred_value = 0;

        Ok(())
    }

    /// Overwrites the address at asssociated `account` index with the given `address`.
    pub fn set_account_address(&mut self, account: usize, address: [u8; 32]) {
        self.0.data_mut().accounts[account].address = address;
    }

    /// Specify the caller account index for the next function or constructor call.
    pub fn set_account(&mut self, index: usize) {
        self.0.data_mut().account = index;
    }

    /// Specify the balance for the next function or constructor call.
    pub fn set_transferred_value(&mut self, amount: u128) {
        self.0.data_mut().transferred_value = amount;
    }

    /// Get the balance of the given `account`.
    pub fn balance(&self, account: usize) -> u128 {
        self.0.data().accounts[account].value
    }

    /// Get the address of the calling account.
    pub fn caller(&self) -> Address {
        self.0.data().accounts[self.0.data().caller_account].address
    }

    /// Get the output of the last function or constructor call.
    pub fn output(&self) -> Vec<u8> {
        if let HostReturn::Data(_, data) = &self.0.data().output {
            return data.to_vec();
        }
        vec![]
    }

    /// Get the debug buffer contents of the last function or constructor call.
    pub fn debug_buffer(&self) -> String {
        self.0.data().debug_buffer.clone()
    }

    /// Get the emitted events of the last function or constructor call.
    pub fn events(&self) -> Vec<Event> {
        self.0.data().events.clone()
    }

    /// Get a list of all deployed contracts.
    pub fn contracts(&self) -> Vec<&Contract> {
        self.0
            .data()
            .accounts
            .iter()
            .map(|a| a.contract.as_ref().unwrap())
            .collect()
    }

    /// Read the storage of the account that was (or is about to be) called.
    pub fn storage(&self) -> &HashMap<StorageKey, Vec<u8>> {
        &self.0.data().accounts[self.0.data().account]
            .contract
            .as_ref()
            .unwrap()
            .storage
    }

    /// Get the selector of the given `function_name` on the given `contract` index.
    pub fn selector(&self, contract: usize, function_name: &str) -> &[u8] {
        &self.0.data().blobs[contract].messages[function_name]
    }

    /// Execute the constructor `index` with the given input `args`.
    pub fn constructor(&mut self, index: usize, mut args: Vec<u8>) {
        let mut input = self.0.data().blobs[self.0.data().account].constructors[index].clone();
        input.append(&mut args);
        self.raw_constructor(input);
    }

    /// Get a list of all uploaded cotracts
    pub fn blobs(&self) -> Vec<WasmCode> {
        self.0.data().blobs.clone()
    }

    /// Call the "deploy" function with the given `input`.
    ///
    /// `input` must contain the selector fo the constructor.
    pub fn raw_constructor(&mut self, input: Vec<u8>) {
        self.invoke("deploy", input).unwrap();
        if let HostReturn::Data(flags, _) = self.0.data().output {
            assert!(flags == 0)
        }
    }

    /// Call the contract function `name` with the given input `args`.
    /// Panics if the contract traps or reverts.
    pub fn function(&mut self, name: &str, mut args: Vec<u8>) {
        let mut input = self.0.data().blobs[self.0.data().account].messages[name].clone();
        input.append(&mut args);
        self.raw_function(input);
    }

    /// Expect the contract function `name` with the given input `args` to trap or revert.
    ///
    /// Only traps caused by an `unreachable` instruction are allowed. Other traps will panic instead.
    pub fn function_expect_failure(&mut self, name: &str, mut args: Vec<u8>) {
        let mut input = self.0.data().blobs[self.0.data().account].messages[name].clone();
        input.append(&mut args);
        self.raw_function_failure(input);
    }

    /// Call the "deploy" function with the given `input`.
    ///
    /// `input` must contain the selector fo the constructor.
    pub fn raw_function(&mut self, input: Vec<u8>) {
        self.invoke("call", input).unwrap();
        if let HostReturn::Data(flags, _) = self.0.data().output {
            assert!(flags == 0)
        }
    }

    fn raw_failure(&mut self, export: &str, input: Vec<u8>) {
        match self.invoke(export, input) {
            Err(wasmi::Error::Trap(trap)) => match trap.trap_code() {
                Some(TrapCode::UnreachableCodeReached) => (),
                _ => panic!("trap: {trap:?}"),
            },
            Err(err) => panic!("unexpected error: {err:?}"),
            Ok(_) => match self.0.data().output {
                HostReturn::Data(1, _) => (),
                _ => panic!("unexpected return from main"),
            },
        }
    }

    /// Call the "call" function with the given input and expect the contract to trap.
    ///
    /// `input` must contain the desired function selector.
    ///
    /// Only traps caused by an `unreachable` instruction are allowed. Other traps will panic instead.
    pub fn raw_function_failure(&mut self, input: Vec<u8>) {
        self.raw_failure("call", input);
    }

    /// Call the "deploy" function with the given input and expect the contract to trap.
    ///
    /// `input` must contain the desired function selector.
    ///
    /// Only traps caused by an `unreachable` instruction are allowed. Other traps will panic instead.
    pub fn raw_constructor_failure(&mut self, input: Vec<u8>) {
        self.raw_failure("deploy", input);
    }

    pub fn heap_verify(&mut self) {
        let mem = self.0.data().memory.unwrap().data(&mut self.0);
        let memsize = mem.len();
        println!("memory size:{memsize}");

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

/// Build all contracts foud in `src` and set up a mock runtime.
///
/// The mock runtime will contain a contract account for each contract in `src`.
/// Constructors are _not_ called, therefore the storage will not be initialized.
pub fn build_solidity(src: &str) -> MockSubstrate {
    build_solidity_with_options(src, true)
}

/// A variant of `MockSubstrate::uild_solidity()` with the ability to specify compiler options:
/// * log_ret: enable logging of host function return codes
/// * log_err: enable logging of runtime errors
pub fn build_solidity_with_options(src: &str, log_err: bool) -> MockSubstrate {
    let blobs = build_wasm(src, log_err)
        .iter()
        .map(|(code, abi)| WasmCode::new(abi, code))
        .collect();

    MockSubstrate(Store::new(&Engine::default(), Runtime::new(blobs)))
}

pub fn build_wasm(src: &str, log_err: bool) -> Vec<(Vec<u8>, String)> {
    let tmp_file = OsStr::new("test.sol");
    let mut cache = FileResolver::default();
    cache.set_file_contents(tmp_file.to_str().unwrap(), src.to_string());
    let opt = inkwell::OptimizationLevel::Default;
    let target = Target::default_polkadot();
    let (wasm, ns) = compile(
        tmp_file,
        &mut cache,
        target,
        &Options {
            opt_level: opt.into(),
            log_runtime_errors: log_err,
            log_prints: true,
            #[cfg(feature = "wasm_opt")]
            wasm_opt: Some(contract_build::OptimizationPasses::Z),
            ..Default::default()
        },
        vec!["unknown".to_string()],
        "0.0.1",
    );
    ns.print_diagnostics_in_plain(&cache, false);
    assert!(!wasm.is_empty());
    wasm
}

pub fn load_abi(s: &str) -> InkProject {
    let bundle = serde_json::from_str::<ContractMetadata>(s).unwrap();
    serde_json::from_value::<InkProject>(serde_json::to_value(bundle.abi).unwrap()).unwrap()
}
