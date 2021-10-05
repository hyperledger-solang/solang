mod solana_helpers;

use base58::{FromBase58, ToBase58};
use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};
use ethabi::{RawLog, Token};
use libc::c_char;
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use solana_helpers::allocator_bump::Allocator;
use solana_rbpf::{
    ebpf,
    error::EbpfError,
    memory_region::{AccessType, MemoryMapping},
    question_mark,
    user_error::UserError,
    vm::{Config, EbpfVm, Executable, SyscallObject, SyscallRegistry, TestInstructionMeter},
};
use solang::{
    abi::generate_abi,
    codegen::{codegen, Options},
    compile_many,
    emit::Generate,
    file_resolver::FileResolver,
    sema::{ast, diagnostics},
    Target,
};
use std::alloc::Layout;
use std::cell::RefCell;
use std::collections::HashMap;
use std::convert::TryInto;
use std::io::Write;
use std::mem::{align_of, size_of};
use std::rc::Rc;
use tiny_keccak::{Hasher, Keccak};

mod solana_tests;

type Account = [u8; 32];

fn account_new() -> Account {
    let mut rng = rand::thread_rng();

    let mut a = [0u8; 32];

    rng.fill(&mut a[..]);

    a
}

struct AccountState {
    data: Vec<u8>,
    owner: Option<Account>,
}

struct VirtualMachine {
    account_data: HashMap<Account, AccountState>,
    programs: Vec<Contract>,
    stack: Vec<Contract>,
    logs: String,
    events: Vec<Vec<Vec<u8>>>,
    return_data: Option<(Account, Vec<u8>)>,
}

#[derive(Clone)]
struct Contract {
    program: Account,
    abi: Option<ethabi::Contract>,
    data: Account,
}

#[derive(Serialize)]
struct ClockLayout {
    slot: u64,
    epoch_start_timestamp: u64,
    epoch: u64,
    leader_schedule_epoch: u64,
    unix_timestamp: u64,
}

#[derive(Deserialize)]
struct CreateAccount {
    instruction: u32,
    _lamports: u64,
    space: u64,
    program_id: Account,
}

#[derive(Deserialize)]
struct CreateAccountWithSeed {
    instruction: u32,
    base: Account,
    seed: String,
    _lamports: u64,
    space: u64,
    program_id: Account,
}

#[derive(Deserialize)]
struct Allocate {
    instruction: u32,
    space: u64,
}

#[derive(Deserialize)]
struct Assign {
    instruction: u32,
    owner: Account,
}

fn build_solidity(src: &str) -> VirtualMachine {
    let mut cache = FileResolver::new();

    cache.set_file_contents("test.sol", src.to_string());

    let mut ns = solang::parse_and_resolve("test.sol", &mut cache, Target::Solana);

    // codegen all the contracts; some additional errors/warnings will be detected here
    codegen(&mut ns, &Options::default());

    diagnostics::print_messages(&cache, &ns, false);

    let context = inkwell::context::Context::create();

    let namespaces = vec![ns];

    let binary = compile_many(
        &context,
        &namespaces,
        "bundle.sol",
        inkwell::OptimizationLevel::Default,
        false,
    );

    let code = binary
        .code(Generate::Linked)
        .expect("llvm code emit should work");

    let mut account_data = HashMap::new();
    let mut programs = Vec::new();

    // resolve
    let ns = &namespaces[0];

    for contract_no in 0..ns.contracts.len() {
        if !ns.contracts[contract_no].is_concrete() {
            continue;
        }

        let (abi, _) = generate_abi(contract_no, ns, &code, false);

        let program = account_new();

        account_data.insert(
            program,
            AccountState {
                data: code.clone(),
                owner: None,
            },
        );

        let abi = ethabi::Contract::load(abi.as_bytes()).unwrap();

        let data = account_new();

        account_data.insert(
            data,
            AccountState {
                data: [0u8; 4096].to_vec(),
                owner: Some(program),
            },
        );

        programs.push(Contract {
            program,
            abi: Some(abi),
            data,
        });
    }

    // Add clock account
    let clock_account: Account = "SysvarC1ock11111111111111111111111111111111"
        .from_base58()
        .unwrap()
        .try_into()
        .unwrap();

    let clock_layout = ClockLayout {
        slot: 70818331,
        epoch: 102,
        epoch_start_timestamp: 946684800,
        leader_schedule_epoch: 1231231312,
        unix_timestamp: 1620656423,
    };

    account_data.insert(
        clock_account,
        AccountState {
            data: bincode::serialize(&clock_layout).unwrap(),
            owner: None,
        },
    );

    let cur = programs.last().unwrap().clone();

    VirtualMachine {
        account_data,
        programs,
        stack: vec![cur],
        logs: String::new(),
        events: Vec::new(),
        return_data: None,
    }
}

const MAX_PERMITTED_DATA_INCREASE: usize = 10 * 1024;

struct AccountRef {
    account: Account,
    offset: usize,
    length: usize,
}

fn serialize_parameters(
    input: &[u8],
    vm: &VirtualMachine,
    seeds: &[&(Account, Vec<u8>)],
) -> (Vec<u8>, Vec<AccountRef>) {
    let mut refs = Vec::new();
    let mut v: Vec<u8> = Vec::new();

    fn serialize_account(
        v: &mut Vec<u8>,
        refs: &mut Vec<AccountRef>,
        key: &Account,
        acc: &AccountState,
    ) {
        // dup_info
        v.write_u8(0xff).unwrap();
        // signer
        v.write_u8(1).unwrap();
        // is_writable
        v.write_u8(1).unwrap();
        // executable
        v.write_u8(1).unwrap();
        // padding
        v.write_all(&[0u8; 4]).unwrap();
        // key
        v.write_all(key).unwrap();
        // owner
        v.write_all(&acc.owner.unwrap_or([0u8; 32])).unwrap();
        // lamports
        v.write_u64::<LittleEndian>(0).unwrap();

        // account data
        v.write_u64::<LittleEndian>(acc.data.len() as u64).unwrap();

        refs.push(AccountRef {
            account: *key,
            offset: v.len(),
            length: acc.data.len(),
        });

        v.write_all(&acc.data).unwrap();
        v.write_all(&[0u8; MAX_PERMITTED_DATA_INCREASE]).unwrap();

        let padding = v.len() % 8;
        if padding != 0 {
            let mut p = Vec::new();
            p.resize(8 - padding, 0);
            v.extend_from_slice(&p);
        }
        // rent epoch
        v.write_u64::<LittleEndian>(0).unwrap();
    }

    // ka_num
    v.write_u64::<LittleEndian>(vm.account_data.len() as u64)
        .unwrap();

    // first do the seeds
    for (acc, _) in seeds {
        let data = &vm.account_data[acc];

        assert!(data.data.is_empty());

        serialize_account(&mut v, &mut refs, acc, data);
    }

    for (acc, data) in &vm.account_data {
        //println!("acc:{} {}", hex::encode(acc), hex::encode(&data.0));
        if !seeds.iter().any(|seed| seed.0 == *acc) {
            serialize_account(&mut v, &mut refs, acc, data);
        }
    }

    // calldata
    v.write_u64::<LittleEndian>(input.len() as u64).unwrap();
    v.write_all(input).unwrap();

    // program id
    v.write_all(&vm.stack[0].program).unwrap();

    (v, refs)
}

// We want to extract the account data
fn deserialize_parameters(
    input: &[u8],
    refs: &[AccountRef],
    accounts_data: &mut HashMap<Account, AccountState>,
) {
    for r in refs {
        if let Some(entry) = accounts_data.get_mut(&r.account) {
            let data = input[r.offset..r.offset + r.length].to_vec();

            entry.data = data;
        }
    }
}

// We want to extract the account data
fn update_parameters(
    input: &[u8],
    refs: &[AccountRef],
    accounts_data: &HashMap<Account, AccountState>,
) {
    for r in refs {
        if let Some(entry) = accounts_data.get(&r.account) {
            unsafe {
                std::ptr::copy(
                    r.length.to_le_bytes().as_ptr(),
                    input[r.offset - 8..].as_ptr() as *mut u8,
                    8,
                );
            }

            unsafe {
                std::ptr::copy(
                    entry.data.as_ptr(),
                    input[r.offset..].as_ptr() as *mut u8,
                    r.length,
                );
            }
        }
    }
}

struct SolLog<'a> {
    context: Rc<RefCell<&'a mut VirtualMachine>>,
}

impl<'a> SyscallObject<UserError> for SolLog<'a> {
    fn call(
        &mut self,
        vm_addr: u64,
        len: u64,
        _arg3: u64,
        _arg4: u64,
        _arg5: u64,
        memory_mapping: &MemoryMapping,
        result: &mut Result<u64, EbpfError<UserError>>,
    ) {
        let host_addr = question_mark!(memory_mapping.map(AccessType::Load, vm_addr, len), result);
        let c_buf: *const c_char = host_addr as *const c_char;
        unsafe {
            for i in 0..len {
                let c = std::ptr::read(c_buf.offset(i as isize));
                if c == 0 {
                    break;
                }
            }
            let message = std::str::from_utf8(std::slice::from_raw_parts(
                host_addr as *const u8,
                len as usize,
            ))
            .unwrap();
            println!("log: {}", message);
            if let Ok(mut context) = self.context.try_borrow_mut() {
                context.logs.push_str(message);
            }
            *result = Ok(0)
        }
    }
}

struct SolLogPubKey<'a> {
    context: Rc<RefCell<&'a mut VirtualMachine>>,
}

impl<'a> SyscallObject<UserError> for SolLogPubKey<'a> {
    fn call(
        &mut self,
        pubkey_addr: u64,
        _arg2: u64,
        _arg3: u64,
        _arg4: u64,
        _arg5: u64,
        memory_mapping: &MemoryMapping,
        result: &mut Result<u64, EbpfError<UserError>>,
    ) {
        let account = question_mark!(
            translate_slice::<Account>(memory_mapping, pubkey_addr, 1),
            result
        );
        let message = account[0].to_base58();
        println!("log pubkey: {}", message);
        if let Ok(mut context) = self.context.try_borrow_mut() {
            context.logs.push_str(&message);
        }
        *result = Ok(0)
    }
}

struct SolSha256();

impl SyscallObject<UserError> for SolSha256 {
    fn call(
        &mut self,
        src: u64,
        len: u64,
        dest: u64,
        _arg4: u64,
        _arg5: u64,
        memory_mapping: &MemoryMapping,
        result: &mut Result<u64, EbpfError<UserError>>,
    ) {
        let arrays = question_mark!(
            translate_slice::<(u64, u64)>(memory_mapping, src, len),
            result
        );

        let mut hasher = Sha256::new();
        for (addr, len) in arrays {
            let buf = question_mark!(translate_slice::<u8>(memory_mapping, *addr, *len), result);
            println!("hashing: {}", hex::encode(buf));
            hasher.update(buf);
        }

        let hash = hasher.finalize();

        let hash_result = question_mark!(
            translate_slice_mut::<u8>(memory_mapping, dest, hash.len() as u64),
            result
        );

        hash_result.copy_from_slice(&hash);

        println!("sol_sha256: {}", hex::encode(hash));

        *result = Ok(0)
    }
}

struct SolKeccak256();

impl SyscallObject<UserError> for SolKeccak256 {
    fn call(
        &mut self,
        src: u64,
        len: u64,
        dest: u64,
        _arg4: u64,
        _arg5: u64,
        memory_mapping: &MemoryMapping,
        result: &mut Result<u64, EbpfError<UserError>>,
    ) {
        let arrays = question_mark!(
            translate_slice::<(u64, u64)>(memory_mapping, src, len),
            result
        );

        let mut hasher = Keccak::v256();
        let mut hash = [0u8; 32];
        for (addr, len) in arrays {
            let buf = question_mark!(translate_slice::<u8>(memory_mapping, *addr, *len), result);
            println!("hashing: {}", hex::encode(buf));
            hasher.update(buf);
        }
        hasher.finalize(&mut hash);

        let hash_result = question_mark!(
            translate_slice_mut::<u8>(memory_mapping, dest, hash.len() as u64),
            result
        );

        hash_result.copy_from_slice(&hash);

        println!("sol_keccak256: {}", hex::encode(hash));

        *result = Ok(0)
    }
}

struct SyscallSetReturnData<'a> {
    context: Rc<RefCell<&'a mut VirtualMachine>>,
}

impl<'a> SyscallObject<UserError> for SyscallSetReturnData<'a> {
    fn call(
        &mut self,
        addr: u64,
        len: u64,
        _arg3: u64,
        _arg4: u64,
        _arg5: u64,
        memory_mapping: &MemoryMapping,
        result: &mut Result<u64, EbpfError<UserError>>,
    ) {
        assert!(len <= 1024, "sol_set_return_data: length is {}", len);

        let buf = question_mark!(translate_slice::<u8>(memory_mapping, addr, len), result);

        if let Ok(mut context) = self.context.try_borrow_mut() {
            if len == 0 {
                context.return_data = None;
            } else {
                context.return_data = Some((context.stack[0].program, buf.to_vec()));
            }

            *result = Ok(0);
        } else {
            panic!();
        }
    }
}

struct SyscallGetReturnData<'a> {
    context: Rc<RefCell<&'a mut VirtualMachine>>,
}

impl<'a> SyscallObject<UserError> for SyscallGetReturnData<'a> {
    fn call(
        &mut self,
        addr: u64,
        len: u64,
        program_id_addr: u64,
        _arg4: u64,
        _arg5: u64,
        memory_mapping: &MemoryMapping,
        result: &mut Result<u64, EbpfError<UserError>>,
    ) {
        if let Ok(context) = self.context.try_borrow() {
            if let Some((program_id, return_data)) = &context.return_data {
                let length = std::cmp::min(len, return_data.len() as u64);

                if len > 0 {
                    let set_result = question_mark!(
                        translate_slice_mut::<u8>(memory_mapping, addr, length),
                        result
                    );

                    set_result.copy_from_slice(&return_data[..length as usize]);

                    let program_id_result = question_mark!(
                        translate_slice_mut::<u8>(memory_mapping, program_id_addr, 32),
                        result
                    );

                    program_id_result.copy_from_slice(program_id);
                }

                *result = Ok(return_data.len() as u64);
            } else {
                *result = Ok(0);
            }
        } else {
            panic!();
        }
    }
}

struct SyscallLogData<'a> {
    context: Rc<RefCell<&'a mut VirtualMachine>>,
}

impl<'a> SyscallObject<UserError> for SyscallLogData<'a> {
    fn call(
        &mut self,
        addr: u64,
        len: u64,
        _arg3: u64,
        _arg4: u64,
        _arg5: u64,
        memory_mapping: &MemoryMapping,
        result: &mut Result<u64, EbpfError<UserError>>,
    ) {
        if let Ok(mut context) = self.context.try_borrow_mut() {
            let untranslated_events =
                question_mark!(translate_slice::<&[u8]>(memory_mapping, addr, len), result);

            let mut events = Vec::with_capacity(untranslated_events.len());

            for untranslated_event in untranslated_events {
                let event = question_mark!(
                    translate_slice_mut::<u8>(
                        memory_mapping,
                        untranslated_event.as_ptr() as u64,
                        untranslated_event.len() as u64,
                    ),
                    result
                );

                events.push(event.to_vec());
            }

            context.events.push(events.to_vec());

            *result = Ok(0);
        } else {
            panic!();
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ed25519SigCheckError {
    InvalidPublicKey,
    InvalidSignature,
    VerifyFailed,
}

impl From<u64> for Ed25519SigCheckError {
    fn from(v: u64) -> Ed25519SigCheckError {
        match v {
            1 => Ed25519SigCheckError::InvalidPublicKey,
            2 => Ed25519SigCheckError::InvalidSignature,
            3 => Ed25519SigCheckError::VerifyFailed,
            _ => panic!("Unsupported Ed25519SigCheckError"),
        }
    }
}

impl From<Ed25519SigCheckError> for u64 {
    fn from(v: Ed25519SigCheckError) -> u64 {
        match v {
            Ed25519SigCheckError::InvalidPublicKey => 1,
            Ed25519SigCheckError::InvalidSignature => 2,
            Ed25519SigCheckError::VerifyFailed => 3,
        }
    }
}

// Shamelessly stolen from solana source

/// Dynamic memory allocation syscall called when the BPF program calls
/// `sol_alloc_free_()`.  The allocator is expected to allocate/free
/// from/to a given chunk of memory and enforce size restrictions.  The
/// memory chunk is given to the allocator during allocator creation and
/// information about that memory (start address and size) is passed
/// to the VM to use for enforcement.
pub struct SyscallAllocFree {
    allocator: Allocator,
}

const DEFAULT_HEAP_SIZE: usize = 32 * 1024;
/// Start of the input buffers in the memory map

impl SyscallObject<UserError> for SyscallAllocFree {
    fn call(
        &mut self,
        size: u64,
        free_addr: u64,
        _arg3: u64,
        _arg4: u64,
        _arg5: u64,
        _memory_mapping: &MemoryMapping,
        result: &mut Result<u64, EbpfError<UserError>>,
    ) {
        let align = align_of::<u128>();
        let layout = match Layout::from_size_align(size as usize, align) {
            Ok(layout) => layout,
            Err(_) => {
                *result = Ok(0);
                return;
            }
        };
        *result = if free_addr == 0 {
            Ok(self.allocator.alloc(layout))
        } else {
            self.allocator.dealloc(free_addr, layout);
            Ok(0)
        }
    }
}

/// Rust representation of C's SolInstruction
#[derive(Debug)]
struct SolInstruction {
    program_id_addr: u64,
    accounts_addr: u64,
    accounts_len: usize,
    data_addr: u64,
    data_len: usize,
}

/// Rust representation of C's SolAccountMeta
#[derive(Debug)]
struct SolAccountMeta {
    pubkey_addr: u64,
    is_writable: bool,
    is_signer: bool,
}

/// Rust representation of C's SolSignerSeed
#[derive(Debug)]
struct SolSignerSeedC {
    addr: u64,
    len: u64,
}

/// Rust representation of C's SolSignerSeeds
#[derive(Debug)]
struct SolSignerSeedsC {
    addr: u64,
    len: u64,
}

#[derive(Debug)]
pub struct Instruction {
    /// Pubkey of the instruction processor that executes this instruction
    pub program_id: Pubkey,
    /// Metadata for what accounts should be passed to the instruction processor
    pub accounts: Vec<AccountMeta>,
    /// Opaque data passed to the instruction processor
    pub data: Vec<u8>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Pubkey([u8; 32]);

impl Pubkey {
    fn is_system_instruction(&self) -> bool {
        self.0 == [0u8; 32]
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct AccountMeta {
    /// An account's public key
    pub pubkey: Pubkey,
    /// True if an Instruction requires a Transaction signature matching `pubkey`.
    pub is_signer: bool,
    /// True if the `pubkey` can be loaded as a read-write account.
    pub is_writable: bool,
}

fn translate(
    memory_mapping: &MemoryMapping,
    access_type: AccessType,
    vm_addr: u64,
    len: u64,
) -> Result<u64, EbpfError<UserError>> {
    memory_mapping.map::<UserError>(access_type, vm_addr, len)
}

fn translate_type_inner<'a, T>(
    memory_mapping: &MemoryMapping,
    access_type: AccessType,
    vm_addr: u64,
) -> Result<&'a mut T, EbpfError<UserError>> {
    unsafe {
        translate(memory_mapping, access_type, vm_addr, size_of::<T>() as u64)
            .map(|value| &mut *(value as *mut T))
    }
}

fn translate_type<'a, T>(
    memory_mapping: &MemoryMapping,
    vm_addr: u64,
) -> Result<&'a T, EbpfError<UserError>> {
    translate_type_inner::<T>(memory_mapping, AccessType::Load, vm_addr).map(|value| &*value)
}

fn translate_slice<'a, T>(
    memory_mapping: &MemoryMapping,
    vm_addr: u64,
    len: u64,
) -> Result<&'a [T], EbpfError<UserError>> {
    translate_slice_inner::<T>(memory_mapping, AccessType::Load, vm_addr, len).map(|value| &*value)
}

fn translate_slice_mut<'a, T>(
    memory_mapping: &MemoryMapping,
    vm_addr: u64,
    len: u64,
) -> Result<&'a mut [T], EbpfError<UserError>> {
    translate_slice_inner::<T>(memory_mapping, AccessType::Store, vm_addr, len)
}

fn translate_slice_inner<'a, T>(
    memory_mapping: &MemoryMapping,
    access_type: AccessType,
    vm_addr: u64,
    len: u64,
) -> Result<&'a mut [T], EbpfError<UserError>> {
    if len == 0 {
        Ok(&mut [])
    } else {
        match translate(
            memory_mapping,
            access_type,
            vm_addr,
            len.saturating_mul(size_of::<T>() as u64),
        ) {
            Ok(value) => {
                Ok(unsafe { std::slice::from_raw_parts_mut(value as *mut T, len as usize) })
            }
            Err(e) => Err(e),
        }
    }
}

struct SyscallInvokeSignedC<'a> {
    context: Rc<RefCell<&'a mut VirtualMachine>>,
    input: &'a [u8],
    refs: Rc<RefCell<&'a mut Vec<AccountRef>>>,
}

impl<'a> SyscallInvokeSignedC<'a> {
    fn translate_instruction(
        &self,
        addr: u64,
        memory_mapping: &MemoryMapping,
    ) -> Result<Instruction, EbpfError<UserError>> {
        let ix_c = translate_type::<SolInstruction>(memory_mapping, addr)?;

        let program_id = translate_type::<Pubkey>(memory_mapping, ix_c.program_id_addr)?;
        let meta_cs = translate_slice::<SolAccountMeta>(
            memory_mapping,
            ix_c.accounts_addr,
            ix_c.accounts_len as u64,
        )?;
        let data =
            translate_slice::<u8>(memory_mapping, ix_c.data_addr, ix_c.data_len as u64)?.to_vec();
        let accounts = meta_cs
            .iter()
            .map(|meta_c| {
                let pubkey = translate_type::<Pubkey>(memory_mapping, meta_c.pubkey_addr)?;
                Ok(AccountMeta {
                    pubkey: pubkey.clone(),
                    is_signer: meta_c.is_signer,
                    is_writable: meta_c.is_writable,
                })
            })
            .collect::<Result<Vec<AccountMeta>, EbpfError<UserError>>>()?;

        Ok(Instruction {
            program_id: program_id.clone(),
            accounts,
            data,
        })
    }
}

fn create_program_address(program_id: &Account, seeds: &[&[u8]]) -> Pubkey {
    let mut hasher = Sha256::new();

    for seed in seeds {
        hasher.update(seed);
    }

    hasher.update(&program_id);
    hasher.update(b"ProgramDerivedAddress");

    let hash = hasher.finalize();

    let new_address: [u8; 32] = hash.try_into().unwrap();

    // the real runtime does checks if this address exists on the ed25519 curve

    Pubkey(new_address)
}

impl<'a> SyscallObject<UserError> for SyscallInvokeSignedC<'a> {
    fn call(
        &mut self,
        instruction_addr: u64,
        _account_infos_addr: u64,
        _account_infos_len: u64,
        signers_seeds_addr: u64,
        signers_seeds_len: u64,
        memory_mapping: &MemoryMapping,
        result: &mut Result<u64, EbpfError<UserError>>,
    ) {
        let instruction = self
            .translate_instruction(instruction_addr, memory_mapping)
            .expect("instruction not valid");

        println!(
            "sol_invoke_signed_c input:{}",
            hex::encode(&instruction.data)
        );

        let seeds = question_mark!(
            translate_slice::<SolSignerSeedsC>(
                memory_mapping,
                signers_seeds_addr,
                signers_seeds_len
            ),
            result
        );

        if let Ok(mut context) = self.context.try_borrow_mut() {
            let signers: Vec<Pubkey> = seeds
                .iter()
                .map(|seed| {
                    let seeds: Vec<&[u8]> =
                        translate_slice::<SolSignerSeedC>(memory_mapping, seed.addr, seed.len)
                            .unwrap()
                            .iter()
                            .map(|seed| {
                                translate_slice::<u8>(memory_mapping, seed.addr, seed.len).unwrap()
                            })
                            .collect();

                    let pda = create_program_address(&context.stack[0].program, &seeds);

                    println!(
                        "pda: {} seeds {}",
                        pda.0.to_base58(),
                        seeds
                            .iter()
                            .map(hex::encode)
                            .collect::<Vec<String>>()
                            .join(" ")
                    );

                    pda
                })
                .collect();

            context.return_data = None;

            if instruction.program_id.is_system_instruction() {
                match bincode::deserialize::<u32>(&instruction.data).unwrap() {
                    0 => {
                        let create_account: CreateAccount =
                            bincode::deserialize(&instruction.data).unwrap();

                        let address = &instruction.accounts[1].pubkey;

                        println!("new address: {}", address.0.to_base58());
                        for s in &signers {
                            println!("signer: {}", s.0.to_base58());
                        }
                        assert!(signers.contains(address));

                        assert_eq!(create_account.instruction, 0);

                        println!(
                            "creating account {} with space {} owner {}",
                            address.0.to_base58(),
                            create_account.space,
                            create_account.program_id.to_base58()
                        );

                        assert_eq!(context.account_data[&address.0].data.len(), 0);

                        if let Some(entry) = context.account_data.get_mut(&address.0) {
                            entry.data = vec![0; create_account.space as usize];
                            entry.owner = Some(create_account.program_id);
                        }

                        let mut refs = self.refs.try_borrow_mut().unwrap();

                        for r in refs.iter_mut() {
                            if r.account == address.0 {
                                r.length = create_account.space as usize;
                            }
                        }
                    }
                    1 => {
                        let assign: Assign = bincode::deserialize(&instruction.data).unwrap();

                        let address = &instruction.accounts[0].pubkey;

                        println!("assign address: {}", address.0.to_base58());
                        for s in &signers {
                            println!("signer: {}", s.0.to_base58());
                        }
                        assert!(signers.contains(address));

                        assert_eq!(assign.instruction, 1);

                        println!(
                            "assign account {} owner {}",
                            address.0.to_base58(),
                            assign.owner.to_base58(),
                        );

                        if let Some(entry) = context.account_data.get_mut(&address.0) {
                            entry.owner = Some(assign.owner);
                        }
                    }
                    3 => {
                        let create_account: CreateAccountWithSeed =
                            bincode::deserialize(&instruction.data).unwrap();

                        assert_eq!(create_account.instruction, 3);

                        let mut hasher = Sha256::new();
                        hasher.update(create_account.base);
                        hasher.update(create_account.seed);
                        hasher.update(create_account.program_id);

                        let hash = hasher.finalize();

                        let new_address: [u8; 32] = hash.try_into().unwrap();

                        println!(
                            "creating account {} with space {} owner {}",
                            hex::encode(new_address),
                            create_account.space,
                            hex::encode(create_account.program_id)
                        );

                        context.account_data.insert(
                            new_address,
                            AccountState {
                                data: vec![0; create_account.space as usize],
                                owner: Some(create_account.program_id),
                            },
                        );

                        context.programs.push(Contract {
                            program: create_account.program_id,
                            abi: None,
                            data: new_address,
                        });
                    }
                    8 => {
                        let allocate: Allocate = bincode::deserialize(&instruction.data).unwrap();

                        let address = &instruction.accounts[0].pubkey;

                        println!("new address: {}", address.0.to_base58());
                        for s in &signers {
                            println!("signer: {}", s.0.to_base58());
                        }
                        assert!(signers.contains(address));

                        assert_eq!(allocate.instruction, 8);

                        println!(
                            "allocate account {} with space {}",
                            address.0.to_base58(),
                            allocate.space,
                        );

                        assert_eq!(context.account_data[&address.0].data.len(), 0);

                        if let Some(entry) = context.account_data.get_mut(&address.0) {
                            entry.data = vec![0; allocate.space as usize];
                        }

                        let mut refs = self.refs.try_borrow_mut().unwrap();

                        for r in refs.iter_mut() {
                            if r.account == address.0 {
                                r.length = allocate.space as usize;
                            }
                        }
                    }
                    instruction => panic!("instruction {} not supported", instruction),
                }
            } else {
                let data_id: Account = instruction.data[..32].try_into().unwrap();

                println!(
                    "calling {} program_id {}",
                    hex::encode(data_id),
                    hex::encode(instruction.program_id.0)
                );

                let p = context
                    .programs
                    .iter()
                    .find(|p| p.program == instruction.program_id.0 && p.data == data_id)
                    .unwrap()
                    .clone();

                context.stack.insert(0, p);

                context.execute(&instruction.data, &[]);

                let refs = self.refs.try_borrow().unwrap();

                update_parameters(self.input, &refs, &context.account_data);

                context.stack.remove(0);
            }
        }

        *result = Ok(0)
    }
}

impl VirtualMachine {
    fn execute(&mut self, calldata: &[u8], seeds: &[&(Account, Vec<u8>)]) {
        println!("running bpf with calldata:{}", hex::encode(calldata));

        let (mut parameter_bytes, mut refs) = serialize_parameters(calldata, self, seeds);
        let mut heap = vec![0_u8; DEFAULT_HEAP_SIZE];

        let program = &self.stack[0];

        let mut syscall_registry = SyscallRegistry::default();
        syscall_registry
            .register_syscall_by_name(b"sol_log_", SolLog::call)
            .unwrap();

        syscall_registry
            .register_syscall_by_name(b"sol_log_pubkey", SolLogPubKey::call)
            .unwrap();

        syscall_registry
            .register_syscall_by_name(b"sol_sha256", SolSha256::call)
            .unwrap();

        syscall_registry
            .register_syscall_by_name(b"sol_keccak256", SolKeccak256::call)
            .unwrap();

        syscall_registry
            .register_syscall_by_name(b"sol_invoke_signed_c", SyscallInvokeSignedC::call)
            .unwrap();

        syscall_registry
            .register_syscall_by_name(b"sol_alloc_free_", SyscallAllocFree::call)
            .unwrap();

        syscall_registry
            .register_syscall_by_name(b"sol_set_return_data", SyscallSetReturnData::call)
            .unwrap();

        syscall_registry
            .register_syscall_by_name(b"sol_get_return_data", SyscallGetReturnData::call)
            .unwrap();

        syscall_registry
            .register_syscall_by_name(b"sol_log_data", SyscallLogData::call)
            .unwrap();

        let executable = <dyn Executable<UserError, TestInstructionMeter>>::from_elf(
            &self.account_data[&program.program].data,
            None,
            Config::default(),
            syscall_registry,
        )
        .expect("should work");

        let mut vm = EbpfVm::<UserError, TestInstructionMeter>::new(
            executable.as_ref(),
            &mut heap,
            &mut parameter_bytes,
        )
        .unwrap();

        let context = Rc::new(RefCell::new(self));
        let refs = Rc::new(RefCell::new(&mut refs));

        vm.bind_syscall_context_object(
            Box::new(SolLog {
                context: context.clone(),
            }),
            None,
        )
        .unwrap();

        vm.bind_syscall_context_object(
            Box::new(SolLogPubKey {
                context: context.clone(),
            }),
            None,
        )
        .unwrap();

        vm.bind_syscall_context_object(
            Box::new(SyscallAllocFree {
                allocator: Allocator::new(DEFAULT_HEAP_SIZE as u64, ebpf::MM_HEAP_START),
            }),
            None,
        )
        .unwrap();

        vm.bind_syscall_context_object(
            Box::new(SyscallInvokeSignedC {
                context: context.clone(),
                input: &parameter_bytes,
                refs: refs.clone(),
            }),
            None,
        )
        .unwrap();

        vm.bind_syscall_context_object(
            Box::new(SyscallSetReturnData {
                context: context.clone(),
            }),
            None,
        )
        .unwrap();

        vm.bind_syscall_context_object(
            Box::new(SyscallGetReturnData {
                context: context.clone(),
            }),
            None,
        )
        .unwrap();

        vm.bind_syscall_context_object(
            Box::new(SyscallLogData {
                context: context.clone(),
            }),
            None,
        )
        .unwrap();

        let res = vm
            .execute_program_interpreted(&mut TestInstructionMeter { remaining: 1000000 })
            .unwrap();

        let mut elf = context.try_borrow_mut().unwrap();
        let refs = refs.try_borrow().unwrap();

        deserialize_parameters(&parameter_bytes, &refs, &mut elf.account_data);

        let output = &elf.account_data[&elf.stack[0].data].data;

        VirtualMachine::validate_heap(output);

        if let Some((_, return_data)) = &elf.return_data {
            println!("return: {}", hex::encode(&return_data));
        }

        assert_eq!(res, 0);
    }

    fn constructor(&mut self, name: &str, args: &[Token], value: u64) {
        let program = &self.stack[0];

        println!("constructor for {}", hex::encode(&program.data));

        let mut calldata = VirtualMachine::input(&program.data, &account_new(), value, name, &[]);

        if let Some(constructor) = &program.abi.as_ref().unwrap().constructor {
            calldata.extend(&constructor.encode_input(vec![], args).unwrap());
        };

        self.execute(&calldata, &[]);
    }

    fn function(
        &mut self,
        name: &str,
        args: &[Token],
        seeds: &[&(Account, Vec<u8>)],
        value: u64,
    ) -> Vec<Token> {
        let program = &self.stack[0];

        println!("function for {}", hex::encode(&program.data));

        let mut calldata = VirtualMachine::input(&program.data, &account_new(), value, name, seeds);

        println!("input: {} seeds {:?}", hex::encode(&calldata), seeds);

        match program.abi.as_ref().unwrap().functions[name][0].encode_input(args) {
            Ok(n) => calldata.extend(&n),
            Err(x) => panic!("{}", x),
        };

        println!("input: {}", hex::encode(&calldata));

        self.execute(&calldata, seeds);

        if let Some((_, return_data)) = &self.return_data {
            println!("return: {}", hex::encode(&return_data));

            let program = &self.stack[0];

            program.abi.as_ref().unwrap().functions[name][0]
                .decode_output(return_data)
                .unwrap()
        } else {
            Vec::new()
        }
    }

    fn input(
        recv: &Account,
        sender: &Account,
        value: u64,
        name: &str,
        seeds: &[&(Account, Vec<u8>)],
    ) -> Vec<u8> {
        let mut calldata: Vec<u8> = recv.to_vec();
        calldata.extend_from_slice(sender);
        calldata.extend_from_slice(&value.to_le_bytes());

        let mut hasher = Keccak::v256();
        let mut hash = [0u8; 32];
        hasher.update(name.as_bytes());
        hasher.finalize(&mut hash);
        calldata.extend(&hash[0..4]);

        let seeds_len = seeds.len() as u8;

        calldata.extend(&[seeds_len]);

        for (_, seed) in seeds {
            let seed_len = seed.len() as u8;

            calldata.extend(&[seed_len]);
            calldata.extend_from_slice(seed);
        }

        calldata
    }

    fn data(&self) -> &Vec<u8> {
        let program = &self.stack[0];

        &self.account_data[&program.data].data
    }

    fn set_program(&mut self, no: usize) {
        let cur = self.programs[no].clone();

        self.stack = vec![cur];
    }

    fn create_empty_account(&mut self) -> (Account, Vec<u8>) {
        let mut rng = rand::thread_rng();

        let mut seed = [0u8; 7];

        rng.fill(&mut seed[..]);

        let pk = create_program_address(&self.stack[0].program, &[&seed]);

        let account = pk.0;

        println!(
            "new empty account {} with seed {}",
            account.to_base58(),
            hex::encode(seed)
        );

        self.account_data.insert(
            account,
            AccountState {
                data: vec![],
                owner: Some([0u8; 32]),
            },
        );

        self.programs.push(Contract {
            program: self.stack[0].program,
            abi: None,
            data: account,
        });

        (account, seed.to_vec())
    }

    fn validate_heap(data: &[u8]) {
        let mut prev_offset = 0;
        let return_len = LittleEndian::read_u32(&data[4..]) as usize;
        let return_offset = LittleEndian::read_u32(&data[8..]) as usize;
        let mut offset = LittleEndian::read_u32(&data[12..]) as usize;

        // println!("data:{}", hex::encode(&data));
        println!("returndata:{}", return_offset);
        let real_return_len = if return_offset == 0 {
            0
        } else {
            LittleEndian::read_u32(&data[return_offset - 8..]) as usize
        };

        assert_eq!(real_return_len, return_len);

        loop {
            let next = LittleEndian::read_u32(&data[offset..]) as usize;
            let prev = LittleEndian::read_u32(&data[offset + 4..]) as usize;
            let length = LittleEndian::read_u32(&data[offset + 8..]) as usize;
            let allocate = LittleEndian::read_u32(&data[offset + 12..]) as usize;

            println!(
                "offset:{} prev:{} next:{} length:{} allocated:{} {}",
                offset,
                prev,
                next,
                length,
                allocate,
                hex::encode(&data[offset + 16..offset + 16 + length])
            );

            assert_eq!(prev, prev_offset);
            prev_offset = offset;

            if next == 0 {
                assert_eq!(length, 0);
                assert_eq!(allocate, 0);

                break;
            }

            let space = next - offset - 16;
            assert!(length <= space);

            offset = next;
        }
    }

    pub fn events(&self) -> Vec<RawLog> {
        self.events
            .iter()
            .map(|fields| {
                assert_eq!(fields.len(), 2);

                assert_eq!(fields[0].len() % 32, 0);
                assert!(fields[0].len() <= 4 * 32);

                let topics = fields[0]
                    .chunks_exact(32)
                    .map(|topic| {
                        let topic: [u8; 32] = topic.try_into().unwrap();

                        ethereum_types::H256::from(topic)
                    })
                    .collect();
                let data = fields[1].clone();

                RawLog { topics, data }
            })
            .collect()
    }
}

pub fn parse_and_resolve(src: &'static str, target: Target) -> ast::Namespace {
    let mut cache = FileResolver::new();

    cache.set_file_contents("test.sol", src.to_string());

    solang::parse_and_resolve("test.sol", &mut cache, target)
}

pub fn first_error(errors: Vec<ast::Diagnostic>) -> String {
    match errors.iter().find(|m| m.level == ast::Level::Error) {
        Some(m) => m.message.to_owned(),
        None => panic!("no errors found"),
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
