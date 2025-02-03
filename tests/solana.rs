// SPDX-License-Identifier: Apache-2.0

use crate::borsh_encoding::{decode_at_offset, encode_arguments, BorshToken};
use anchor_syn::idl::types::{Idl, IdlAccountItem};
use base58::{FromBase58, ToBase58};
use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};
use itertools::Itertools;
use libc::c_char;
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use solana_rbpf::{
    aligned_memory::AlignedMemory,
    ebpf,
    elf::{Executable, SBPFVersion},
    error::EbpfError,
    memory_region::{AccessType, MemoryMapping, MemoryRegion},
    verifier::{RequisiteVerifier, TautologyVerifier},
    vm::{BuiltinProgram, Config, ContextObject, EbpfVm, ProgramResult, StableResult},
};
use solang::abi::anchor::function_discriminator;
use solang::{
    abi::anchor::generate_anchor_idl,
    codegen::{OptimizationLevel, Options},
    compile,
    file_resolver::FileResolver,
    sema::ast,
    Target,
};
use std::{
    cell::{RefCell, RefMut},
    collections::HashMap,
    convert::TryInto,
    ffi::OsStr,
    io::Write,
    mem::size_of,
    rc::Rc,
    sync::Arc,
};
use tiny_keccak::{Hasher, Keccak};

mod borsh_encoding;
mod solana_tests;

type Error = Box<dyn std::error::Error>;

/// Error handling for syscall methods
macro_rules! question_mark {
    ( $value:expr, $result:ident ) => {{
        let value = $value;
        match value {
            Err(err) => {
                *$result = ProgramResult::Err(err);
                return;
            }
            Ok(value) => value,
        }
    }};
}

pub type Account = [u8; 32];

pub fn account_new() -> Account {
    let mut rng = rand::thread_rng();

    let mut a = [0u8; 32];

    rng.fill(&mut a[..]);

    a
}

#[derive(Default)]
struct AccountState {
    data: Vec<u8>,
    owner: Option<Account>,
    lamports: u64,
}

/// We have a special callback function which tests that the correct
/// parameters are passed in during CPI.
type CallParametersCheck = fn(vm: &VirtualMachine, instr: &Instruction, pda: &[Pubkey]);

struct VirtualMachine {
    account_data: HashMap<Account, AccountState>,
    programs: Vec<Program>,
    stack: Vec<Program>,
    logs: String,
    events: Vec<Vec<Vec<u8>>>,
    return_data: Option<(Account, Vec<u8>)>,
    call_params_check: HashMap<Pubkey, CallParametersCheck>,
}

#[derive(Clone)]
struct Program {
    id: Account,
    idl: Option<Idl>,
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
    VirtualMachineBuilder::new(src).build()
}

fn build_solidity_with_cache(cache: FileResolver) -> VirtualMachine {
    VirtualMachineBuilder::new_with_cache(cache).build()
}

pub(crate) struct VirtualMachineBuilder {
    cache: FileResolver,
    opts: Option<Options>,
}

impl VirtualMachineBuilder {
    pub(crate) fn new(src: &str) -> Self {
        let mut cache = FileResolver::default();
        cache.set_file_contents("test.sol", src.to_string());
        Self { cache, opts: None }
    }

    pub(crate) fn new_with_cache(cache: FileResolver) -> Self {
        Self { cache, opts: None }
    }

    pub(crate) fn opts(mut self, opts: Options) -> Self {
        self.opts = Some(opts);
        self
    }

    pub(crate) fn build(mut self) -> VirtualMachine {
        let (res, ns) = compile(
            OsStr::new("test.sol"),
            &mut self.cache,
            Target::Solana,
            self.opts.as_ref().unwrap_or(&Options {
                opt_level: OptimizationLevel::Default,
                log_runtime_errors: true,
                log_prints: true,
                ..Default::default()
            }),
            vec!["unknown".to_string()],
            "0.0.1",
        );

        ns.print_diagnostics_in_plain(&self.cache, false);

        assert!(!res.is_empty());

        let mut account_data = HashMap::new();
        let mut programs = Vec::new();

        for contract_no in 0..ns.contracts.len() {
            let contract = &ns.contracts[contract_no];

            if !contract.instantiable {
                continue;
            }

            let code = contract.code.get().unwrap();
            let idl = generate_anchor_idl(contract_no, &ns, "0.1.0");

            let program = if let Some(program_id) = &contract.program_id {
                program_id.clone().try_into().unwrap()
            } else {
                account_new()
            };

            account_data.insert(
                program,
                AccountState {
                    data: code.clone(),
                    owner: None,
                    lamports: 0,
                },
            );

            programs.push(Program {
                id: program,
                idl: Some(idl),
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
                lamports: 0,
            },
        );

        account_data.insert([0; 32], AccountState::default());

        let cur = programs.last().unwrap().clone();

        VirtualMachine {
            account_data,
            programs,
            stack: vec![cur],
            logs: String::new(),
            events: Vec::new(),
            return_data: None,
            call_params_check: HashMap::new(),
        }
    }
}

const MAX_PERMITTED_DATA_INCREASE: usize = 10 * 1024;

struct AccountRef {
    account: Account,
    owner_offset: usize,
    data_offset: usize,
    length: usize,
}

enum SerializableAccount {
    Unique(AccountMeta),
    Duplicate(usize),
}

fn remove_duplicates(metas: &[AccountMeta]) -> Vec<SerializableAccount> {
    let mut serializable_format: Vec<SerializableAccount> = Vec::new();
    let mut inserted: HashMap<AccountMeta, usize> = HashMap::new();

    for (idx, account) in metas.iter().enumerate() {
        if let Some(idx) = inserted.get(account) {
            serializable_format.push(SerializableAccount::Duplicate(*idx));
        } else {
            serializable_format.push(SerializableAccount::Unique(account.clone()));
            inserted.insert(account.clone(), idx);
        }
    }
    serializable_format
}

fn serialize_parameters(
    input: &[u8],
    metas: &[AccountMeta],
    vm: &VirtualMachine,
) -> (Vec<u8>, Vec<AccountRef>) {
    let mut refs = Vec::new();
    let mut v: Vec<u8> = Vec::new();

    fn serialize_account(
        v: &mut Vec<u8>,
        refs: &mut Vec<AccountRef>,
        meta: &AccountMeta,
        acc: &AccountState,
    ) {
        // dup_info
        v.write_u8(0xff).unwrap();
        // signer
        v.write_u8(meta.is_signer.into()).unwrap();
        // is_writable
        v.write_u8(meta.is_writable.into()).unwrap();
        // executable
        v.write_u8(1).unwrap();
        // padding
        v.write_all(&[0u8; 4]).unwrap();
        // key
        v.write_all(&meta.pubkey.0).unwrap();
        // owner
        let owner_offset = v.len();

        v.write_all(&acc.owner.unwrap_or([0u8; 32])).unwrap();
        // lamports
        v.write_u64::<LittleEndian>(acc.lamports).unwrap();

        // account data
        v.write_u64::<LittleEndian>(acc.data.len() as u64).unwrap();

        refs.push(AccountRef {
            account: meta.pubkey.0,
            owner_offset,
            data_offset: v.len(),
            length: acc.data.len(),
        });

        v.write_all(&acc.data).unwrap();
        v.write_all(&[0u8; MAX_PERMITTED_DATA_INCREASE]).unwrap();

        let padding = v.len() % 8;
        if padding != 0 {
            let p = vec![0; 8 - padding];
            v.extend_from_slice(&p);
        }
        // rent epoch
        v.write_u64::<LittleEndian>(0).unwrap();
    }

    let no_duplicates_meta = remove_duplicates(metas);
    // ka_num
    v.write_u64::<LittleEndian>(no_duplicates_meta.len() as u64)
        .unwrap();

    for account_item in &no_duplicates_meta {
        match account_item {
            SerializableAccount::Unique(account) => {
                serialize_account(
                    &mut v,
                    &mut refs,
                    account,
                    &vm.account_data[&account.pubkey.0],
                );
            }
            SerializableAccount::Duplicate(idx) => {
                v.write_u64::<LittleEndian>(*idx as u64).unwrap();
            }
        }
    }

    // calldata
    v.write_u64::<LittleEndian>(input.len() as u64).unwrap();
    v.write_all(input).unwrap();

    // program id
    v.write_all(&vm.stack[0].id).unwrap();

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
            let data = input[r.data_offset..r.data_offset + r.length].to_vec();

            entry.data = data;
            entry.lamports = u64::from_ne_bytes(
                input[r.data_offset - 16..r.data_offset - 8]
                    .try_into()
                    .unwrap(),
            );
        }
    }
}

// We want to extract the account data
fn update_parameters(
    input: &[u8],
    mut refs: RefMut<&mut Vec<AccountRef>>,
    accounts_data: &HashMap<Account, AccountState>,
) {
    for r in refs.iter_mut() {
        if let Some(entry) = accounts_data.get(&r.account) {
            r.length = entry.data.len();
            unsafe {
                std::ptr::copy(
                    r.length.to_le_bytes().as_ptr(),
                    input[r.data_offset - 8..].as_ptr() as *mut u8,
                    8,
                );
            }

            unsafe {
                std::ptr::copy(
                    entry.data.as_ptr(),
                    input[r.data_offset..].as_ptr() as *mut u8,
                    r.length,
                );
            }

            if let Some(owner) = &entry.owner {
                unsafe {
                    std::ptr::copy(
                        owner.as_ptr(),
                        input[r.owner_offset..].as_ptr() as *mut u8,
                        32,
                    );
                }
            }
        }
    }
}

#[derive(Clone)]
struct SyscallContext<'a> {
    vm: Rc<RefCell<&'a mut VirtualMachine>>,
    input_len: usize,
    refs: Rc<RefCell<&'a mut Vec<AccountRef>>>,
    heap: *const u8,
    pub remaining: u64,
}

impl ContextObject for SyscallContext<'_> {
    fn trace(&mut self, _state: [u64; 12]) {}

    fn consume(&mut self, amount: u64) {
        debug_assert!(amount <= self.remaining, "Execution count exceeded");
        self.remaining = self.remaining.saturating_sub(amount);
    }

    fn get_remaining(&self) -> u64 {
        self.remaining
    }
}

impl SyscallContext<'_> {
    pub fn heap_verify(&self) {
        const VERBOSE: bool = false;

        let heap: &[u8] = unsafe { std::slice::from_raw_parts(self.heap, DEFAULT_HEAP_SIZE) };

        const HEAP_START: u64 = 0x3_0000_0000;
        let mut current_elem = HEAP_START;
        let mut last_elem = 0;

        let read_u64 = |offset: u64| {
            let offset = (offset - HEAP_START) as usize;
            u64::from_le_bytes(heap[offset..offset + 8].try_into().unwrap())
        };

        let read_u32 = |offset: u64| {
            let offset = (offset - HEAP_START) as usize;
            u32::from_le_bytes(heap[offset..offset + 4].try_into().unwrap())
        };

        if VERBOSE {
            println!("heap verify:");
        }

        loop {
            let next: u64 = read_u64(current_elem);
            let prev: u64 = read_u64(current_elem + 8);
            let length: u32 = read_u32(current_elem + 16);
            let allocated: u32 = read_u32(current_elem + 20);

            if VERBOSE {
                println!("next:{next:08x} prev:{prev:08x} length:{length} allocated:{allocated}");
            }

            let start = (current_elem + 24 - HEAP_START) as usize;

            let buf = &heap[start..start + length as usize];

            if allocated == 0 {
                if VERBOSE {
                    println!("{:08x} {} not allocated", current_elem + 32, length);
                }
            } else {
                if VERBOSE {
                    println!("{:08x} {} allocated", current_elem + 32, length);
                }

                assert_eq!(allocated & 0xffff, 1);

                for offset in (0..buf.len()).step_by(16) {
                    use std::fmt::Write;

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
                    if VERBOSE {
                        println!("{hex}\n{chars}");
                    }
                }
            }

            assert_eq!(last_elem, prev);

            if next == 0 {
                break;
            }

            last_elem = current_elem;
            current_elem = next;
        }

        if VERBOSE {
            println!("heap verify done");
        }
    }
}

fn sol_panic_(
    _context: &mut SyscallContext,
    _src: u64,
    _len: u64,
    _dest: u64,
    _arg4: u64,
    _arg5: u64,
    _memory_mapping: &mut MemoryMapping,
    result: &mut ProgramResult,
) {
    println!("sol_panic_()");
    *result = ProgramResult::Err(EbpfError::ExecutionOverrun(0).into());
}

fn sol_log(
    context: &mut SyscallContext,
    vm_addr: u64,
    len: u64,
    _arg3: u64,
    _arg4: u64,
    _arg5: u64,
    memory_mapping: &mut MemoryMapping,
    result: &mut ProgramResult,
) {
    context.heap_verify();

    let host_addr = memory_mapping
        .map(AccessType::Load, vm_addr, len, 0)
        .unwrap();
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
        println!("log: {message}");
        if let Ok(mut vm) = context.vm.try_borrow_mut() {
            vm.logs.push_str(message);
        }
        *result = ProgramResult::Ok(0)
    }
}

fn sol_log_pubkey(
    context: &mut SyscallContext,
    pubkey_addr: u64,
    _arg2: u64,
    _arg3: u64,
    _arg4: u64,
    _arg5: u64,
    memory_mapping: &mut MemoryMapping,
    result: &mut ProgramResult,
) {
    context.heap_verify();

    let account = translate_slice::<Account>(memory_mapping, pubkey_addr, 1).unwrap();
    let message = account[0].to_base58();
    println!("log pubkey: {message}");
    if let Ok(mut vm) = context.vm.try_borrow_mut() {
        vm.logs.push_str(&message);
    }
    *result = ProgramResult::Ok(0)
}

fn sol_log_u64(
    context: &mut SyscallContext,
    arg1: u64,
    arg2: u64,
    arg3: u64,
    arg4: u64,
    arg5: u64,
    _memory_mapping: &mut MemoryMapping,
    result: &mut ProgramResult,
) {
    let message = format!("{arg1:#x}, {arg2:#x}, {arg3:#x}, {arg4:#x}, {arg5:#x}");

    println!("log64: {message}");

    context.heap_verify();

    if let Ok(mut vm) = context.vm.try_borrow_mut() {
        vm.logs.push_str(&message);
    }
    *result = ProgramResult::Ok(0)
}

fn sol_sha256(
    context: &mut SyscallContext,
    src: u64,
    len: u64,
    dest: u64,
    _arg4: u64,
    _arg5: u64,
    memory_mapping: &mut MemoryMapping,
    result: &mut ProgramResult,
) {
    context.heap_verify();

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

    *result = ProgramResult::Ok(0)
}

fn sol_keccak256(
    context: &mut SyscallContext,
    src: u64,
    len: u64,
    dest: u64,
    _arg4: u64,
    _arg5: u64,
    memory_mapping: &mut MemoryMapping,
    result: &mut ProgramResult,
) {
    context.heap_verify();

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

    *result = ProgramResult::Ok(0)
}

fn sol_create_program_address(
    _context: &mut SyscallContext,
    seed_ptr: u64,
    seed_len: u64,
    program_id: u64,
    dest: u64,
    _arg5: u64,
    memory_mapping: &mut MemoryMapping,
    result: &mut ProgramResult,
) {
    assert!(seed_len <= 16);

    let arrays = question_mark!(
        translate_slice::<(u64, u64)>(memory_mapping, seed_ptr, seed_len),
        result
    );

    let mut seeds = Vec::new();

    for (addr, len) in arrays {
        assert!(*len < 32);

        let buf = question_mark!(translate_slice::<u8>(memory_mapping, *addr, *len), result);

        println!("seed:{}", hex::encode(buf));

        seeds.push(buf);
    }

    let program_id = question_mark!(
        translate_type::<Account>(memory_mapping, program_id),
        result
    );

    println!("program_id:{}", program_id.to_base58());

    let pda = create_program_address(program_id, &seeds);

    let hash_result = question_mark!(translate_slice_mut::<u8>(memory_mapping, dest, 32), result);

    hash_result.copy_from_slice(&pda.0);

    println!("sol_create_program_address: {}", pda.0.to_base58());

    *result = ProgramResult::Ok(0)
}

fn sol_try_find_program_address(
    _context: &mut SyscallContext,
    seed_ptr: u64,
    seed_len: u64,
    program_id: u64,
    dest: u64,
    bump: u64,
    memory_mapping: &mut MemoryMapping,
    result: &mut ProgramResult,
) {
    assert!(seed_len <= 16);

    let arrays = question_mark!(
        translate_slice::<(u64, u64)>(memory_mapping, seed_ptr, seed_len),
        result
    );

    let mut seeds = Vec::new();

    for (addr, len) in arrays {
        assert!(*len < 32);

        let buf = translate_slice::<u8>(memory_mapping, *addr, *len).unwrap();

        println!("seed:{}", hex::encode(buf));

        seeds.push(buf);
    }

    let program_id = question_mark!(
        translate_type::<Account>(memory_mapping, program_id),
        result
    );

    println!("program_id:{}", program_id.to_base58());

    let bump_seed = [u8::MAX];
    let mut seeds_with_bump = seeds.to_vec();
    seeds_with_bump.push(&bump_seed);

    let pda = create_program_address(program_id, &seeds_with_bump);

    let hash_result = question_mark!(translate_slice_mut::<u8>(memory_mapping, dest, 32), result);

    hash_result.copy_from_slice(&pda.0);

    let bump_result = question_mark!(translate_slice_mut::<u8>(memory_mapping, bump, 1), result);

    bump_result.copy_from_slice(&bump_seed);

    println!(
        "sol_try_find_program_address: {} {:x}",
        pda.0.to_base58(),
        bump_seed[0]
    );

    *result = ProgramResult::Ok(0)
}

fn sol_set_return_data(
    context: &mut SyscallContext,
    addr: u64,
    len: u64,
    _arg3: u64,
    _arg4: u64,
    _arg5: u64,
    memory_mapping: &mut MemoryMapping,
    result: &mut ProgramResult,
) {
    context.heap_verify();

    assert!(len <= 1024, "sol_set_return_data: length is {len}");

    let buf = question_mark!(translate_slice::<u8>(memory_mapping, addr, len), result);

    println!("sol_set_return_data: {}", hex::encode(buf));

    if let Ok(mut vm) = context.vm.try_borrow_mut() {
        if len == 0 {
            vm.return_data = None;
        } else {
            vm.return_data = Some((vm.stack[0].id, buf.to_vec()));
        }

        *result = ProgramResult::Ok(0);
    } else {
        panic!();
    }
}

fn sol_get_return_data(
    context: &mut SyscallContext,
    addr: u64,
    len: u64,
    program_id_addr: u64,
    _arg4: u64,
    _arg5: u64,
    memory_mapping: &mut MemoryMapping,
    result: &mut ProgramResult,
) {
    context.heap_verify();

    if let Ok(vm) = context.vm.try_borrow() {
        if let Some((program_id, return_data)) = &vm.return_data {
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

            *result = ProgramResult::Ok(return_data.len() as u64);
        } else {
            *result = ProgramResult::Ok(0);
        }
    } else {
        panic!();
    }
}

fn sol_log_data(
    context: &mut SyscallContext,
    addr: u64,
    len: u64,
    _arg3: u64,
    _arg4: u64,
    _arg5: u64,
    memory_mapping: &mut MemoryMapping,
    result: &mut ProgramResult,
) {
    context.heap_verify();

    if let Ok(mut vm) = context.vm.try_borrow_mut() {
        print!("sol_log_data");
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

            print!(" {}", hex::encode(event));
        }

        println!();

        vm.events.push(events.to_vec());

        *result = ProgramResult::Ok(0);
    } else {
        panic!();
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

const DEFAULT_HEAP_SIZE: usize = 32 * 1024;

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

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Pubkey([u8; 32]);

impl Pubkey {
    fn is_system_instruction(&self) -> bool {
        self.0 == [0u8; 32]
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct AccountMeta {
    /// An account's public key
    pub pubkey: Pubkey,
    /// True if the `pubkey` can be loaded as a read-write account.
    pub is_writable: bool,
    /// True if an Instruction requires a Transaction signature matching `pubkey`.
    pub is_signer: bool,
}

fn translate(
    memory_mapping: &MemoryMapping,
    access_type: AccessType,
    vm_addr: u64,
    len: u64,
) -> Result<u64, Error> {
    memory_mapping.map(access_type, vm_addr, len, 0).into()
}

fn translate_type_inner<'a, T>(
    memory_mapping: &MemoryMapping,
    access_type: AccessType,
    vm_addr: u64,
) -> Result<&'a mut T, Error> {
    let host_addr = translate(memory_mapping, access_type, vm_addr, size_of::<T>() as u64)?;

    // host_addr is in our address space, cast
    Ok(unsafe { &mut *(host_addr as *mut T) })
}

fn translate_type<'a, T>(memory_mapping: &MemoryMapping, vm_addr: u64) -> Result<&'a T, Error> {
    translate_type_inner::<T>(memory_mapping, AccessType::Load, vm_addr).map(|value| &*value)
}

fn translate_slice<'a, T>(
    memory_mapping: &MemoryMapping,
    vm_addr: u64,
    len: u64,
) -> Result<&'a [T], Error> {
    translate_slice_inner::<T>(memory_mapping, AccessType::Load, vm_addr, len).map(|value| &*value)
}

fn translate_slice_mut<'a, T>(
    memory_mapping: &MemoryMapping,
    vm_addr: u64,
    len: u64,
) -> Result<&'a mut [T], Error> {
    translate_slice_inner::<T>(memory_mapping, AccessType::Store, vm_addr, len)
}

fn translate_slice_inner<'a, T>(
    memory_mapping: &MemoryMapping,
    access_type: AccessType,
    vm_addr: u64,
    len: u64,
) -> Result<&'a mut [T], Error> {
    if len == 0 {
        return Ok(&mut []);
    }

    let total_size = len.saturating_mul(size_of::<T>() as u64);

    let host_addr = translate(memory_mapping, access_type, vm_addr, total_size)?;

    // host_addr is in our address space, cast
    Ok(unsafe { std::slice::from_raw_parts_mut(host_addr as *mut T, len as usize) })
}

fn translate_instruction(addr: u64, memory_mapping: &MemoryMapping) -> Result<Instruction, Error> {
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
        .collect::<Result<Vec<AccountMeta>, Error>>()?;

    Ok(Instruction {
        program_id: program_id.clone(),
        accounts,
        data,
    })
}

fn create_program_address(program_id: &Account, seeds: &[&[u8]]) -> Pubkey {
    let mut hasher = Sha256::new();

    for seed in seeds {
        hasher.update(seed);
    }

    hasher.update(program_id);
    hasher.update(b"ProgramDerivedAddress");

    let hash = hasher.finalize();

    let new_address: [u8; 32] = hash.into();

    // the real runtime does checks if this address exists on the ed25519 curve

    Pubkey(new_address)
}

fn sol_invoke_signed_c(
    context: &mut SyscallContext,
    instruction_addr: u64,
    _account_infos_addr: u64,
    _account_infos_len: u64,
    signers_seeds_addr: u64,
    signers_seeds_len: u64,
    memory_mapping: &mut MemoryMapping,
    result: &mut ProgramResult,
) {
    let instruction =
        translate_instruction(instruction_addr, memory_mapping).expect("instruction not valid");

    println!(
        "sol_invoke_signed_c input:{}",
        hex::encode(&instruction.data)
    );

    let seeds = question_mark!(
        translate_slice::<SolSignerSeedsC>(memory_mapping, signers_seeds_addr, signers_seeds_len),
        result
    );

    if let Ok(mut vm) = context.vm.try_borrow_mut() {
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

                let pda = create_program_address(&vm.stack[0].id, &seeds);

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

        vm.return_data = None;

        if let Some(handle) = vm.call_params_check.get(&instruction.program_id) {
            handle(&vm, &instruction, &signers);
        } else if instruction.program_id.is_system_instruction() {
            match bincode::deserialize::<u32>(&instruction.data).unwrap() {
                0 => {
                    let create_account: CreateAccount =
                        bincode::deserialize(&instruction.data).unwrap();

                    let address = &instruction.accounts[1].pubkey;

                    assert!(instruction.accounts[1].is_signer);

                    println!("new address: {}", address.0.to_base58());
                    for s in &signers {
                        println!("signer: {}", s.0.to_base58());
                    }

                    if !signers.is_empty() {
                        assert!(signers.contains(address));
                    }

                    assert_eq!(create_account.instruction, 0);

                    println!(
                        "creating account {} with space {} owner {}",
                        address.0.to_base58(),
                        create_account.space,
                        create_account.program_id.to_base58()
                    );

                    assert_eq!(vm.account_data[&address.0].data.len(), 0);

                    if let Some(entry) = vm.account_data.get_mut(&address.0) {
                        entry.data = vec![0; create_account.space as usize];
                        entry.owner = Some(create_account.program_id);
                    }

                    let mut refs = context.refs.try_borrow_mut().unwrap();

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

                    if let Some(entry) = vm.account_data.get_mut(&address.0) {
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

                    let new_address: [u8; 32] = hash.into();

                    println!(
                        "creating account {} with space {} owner {}",
                        hex::encode(new_address),
                        create_account.space,
                        hex::encode(create_account.program_id)
                    );

                    vm.account_data.insert(
                        new_address,
                        AccountState {
                            data: vec![0; create_account.space as usize],
                            owner: Some(create_account.program_id),
                            lamports: 0,
                        },
                    );

                    vm.programs.push(Program {
                        id: create_account.program_id,
                        idl: None,
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

                    assert_eq!(vm.account_data[&address.0].data.len(), 0);

                    if let Some(entry) = vm.account_data.get_mut(&address.0) {
                        entry.data = vec![0; allocate.space as usize];
                    }

                    let mut refs = context.refs.try_borrow_mut().unwrap();

                    for r in refs.iter_mut() {
                        if r.account == address.0 {
                            r.length = allocate.space as usize;
                        }
                    }
                }
                instruction => panic!("instruction {instruction} not supported"),
            }
        } else {
            println!(
                "calling program_id {}",
                instruction.program_id.0.to_base58()
            );

            let p = vm
                .programs
                .iter()
                .find(|p| p.id == instruction.program_id.0)
                .unwrap()
                .clone();

            vm.stack.insert(0, p);

            let res = vm.execute(&instruction.accounts, &instruction.data);
            assert!(matches!(res, StableResult::Ok(0)), "external call failed");

            let refs = context.refs.try_borrow_mut().unwrap();

            let input = translate_slice_mut::<u8>(
                memory_mapping,
                ebpf::MM_INPUT_START,
                context.input_len as u64,
            )
            .unwrap();

            update_parameters(input, refs, &vm.account_data);

            vm.stack.remove(0);
        }
    }

    *result = ProgramResult::Ok(0)
}

impl VirtualMachine {
    fn execute(&mut self, metas: &[AccountMeta], calldata: &[u8]) -> ProgramResult {
        println!("running bpf with calldata:{}", hex::encode(calldata));

        let (mut parameter_bytes, mut refs) = serialize_parameters(calldata, metas, self);
        let mut heap = vec![0_u8; DEFAULT_HEAP_SIZE];

        let program = &self.stack[0];
        let config = Config {
            enable_sbpf_v1: true,
            enable_symbol_and_section_labels: false,
            ..Config::default()
        };

        let mut loader = BuiltinProgram::new_loader(config);

        loader.register_function(b"sol_panic_", sol_panic_).unwrap();

        loader.register_function(b"sol_log_", sol_log).unwrap();

        loader
            .register_function(b"sol_log_pubkey", sol_log_pubkey)
            .unwrap();

        loader
            .register_function(b"sol_log_64_", sol_log_u64)
            .unwrap();

        loader.register_function(b"sol_sha256", sol_sha256).unwrap();

        loader
            .register_function(b"sol_keccak256", sol_keccak256)
            .unwrap();

        loader
            .register_function(b"sol_create_program_address", sol_create_program_address)
            .unwrap();

        loader
            .register_function(
                b"sol_try_find_program_address",
                sol_try_find_program_address,
            )
            .unwrap();

        loader
            .register_function(b"sol_invoke_signed_c", sol_invoke_signed_c)
            .unwrap();

        loader
            .register_function(b"sol_set_return_data", sol_set_return_data)
            .unwrap();

        loader
            .register_function(b"sol_get_return_data", sol_get_return_data)
            .unwrap();

        loader
            .register_function(b"sol_log_data", sol_log_data)
            .unwrap();

        // program.program
        println!("program: {}", program.id.to_base58());

        let executable = Executable::<TautologyVerifier, SyscallContext>::from_elf(
            &self.account_data[&program.id].data,
            Arc::new(loader),
        )
        .expect("should work");
        let config = *executable.get_config();
        let text = executable.get_ro_region();

        let verified_executable =
            Executable::<RequisiteVerifier, SyscallContext>::verified(executable).unwrap();

        let mut context = SyscallContext {
            vm: Rc::new(RefCell::new(self)),
            input_len: parameter_bytes.len(),
            refs: Rc::new(RefCell::new(&mut refs)),
            heap: heap.as_ptr(),
            remaining: 1000000,
        };

        let mut stack = AlignedMemory::<{ ebpf::HOST_ALIGN }>::zero_filled(config.stack_size());

        let parameter_region = vec![
            text,
            MemoryRegion::new_writable(&mut parameter_bytes, ebpf::MM_INPUT_START),
            MemoryRegion::new_writable(&mut heap, ebpf::MM_HEAP_START),
            MemoryRegion::new_writable(stack.as_slice_mut(), ebpf::MM_STACK_START),
        ];

        let memory_mapping =
            MemoryMapping::new(parameter_region, &config, &SBPFVersion::V1).unwrap();

        let mut vm = EbpfVm::new(
            &config,
            &SBPFVersion::V1,
            &mut context,
            memory_mapping,
            4196,
        );

        let (_, res) = vm.execute_program(&verified_executable, true);

        deserialize_parameters(&parameter_bytes, &refs, &mut self.account_data);

        if let Some((_, return_data)) = &self.return_data {
            println!("return: {}", hex::encode(return_data));
        }

        res
    }

    fn function(&mut self, name: &str) -> VmFunction {
        let idx = if let Some((idx, _)) = self.stack[0]
            .idl
            .as_ref()
            .unwrap()
            .instructions
            .iter()
            .find_position(|instr| instr.name == name)
        {
            idx
        } else {
            panic!("Function not found")
        };

        VmFunction {
            vm: self,
            idx,
            expected: 0,
            accounts: Vec::new(),
            has_remaining: false,
            arguments: None,
            data_account: None,
        }
    }

    fn set_program(&mut self, no: usize) {
        let cur = self.programs[no].clone();

        self.stack = vec![cur];
    }

    fn create_pda(&mut self, program_id: &Account, len: usize) -> (Account, Vec<u8>) {
        let mut rng = rand::thread_rng();

        let mut seed = vec![0u8; len];

        rng.fill(&mut seed[..]);

        let pk = create_program_address(program_id, &[&seed]);

        let account = pk.0;

        println!(
            "new empty account {} with seed {}",
            account.to_base58(),
            hex::encode(&seed)
        );

        self.create_empty_account(&account, program_id);

        (account, seed)
    }

    fn create_empty_account(&mut self, account: &Account, program_id: &Account) {
        self.account_data.insert(
            *account,
            AccountState {
                data: vec![],
                owner: Some([0u8; 32]),
                lamports: 0,
            },
        );

        self.programs.push(Program {
            id: *program_id,
            idl: None,
        });
    }

    fn validate_account_data_heap(&self, account: &Pubkey) -> usize {
        if let Some(acc) = self.account_data.get(&account.0) {
            let data = &acc.data;

            let mut count = 0;

            if data.len() < 4 || LittleEndian::read_u32(&data[0..]) == 0 {
                return count;
            }

            let mut prev_offset = 0;
            let return_len = LittleEndian::read_u32(&data[4..]) as usize;
            let return_offset = LittleEndian::read_u32(&data[8..]) as usize;
            let mut offset = LittleEndian::read_u32(&data[12..]) as usize;

            // The return_offset/len fields are no longer used (we should remove them at some point)
            assert_eq!(return_len, 0);
            assert_eq!(return_offset, 0);

            println!(
                "static: length:{:x} {}",
                offset - 16,
                hex::encode(&data[16..offset])
            );

            if offset >= data.len() {
                return count;
            }

            loop {
                let next = LittleEndian::read_u32(&data[offset..]) as usize;
                let prev = LittleEndian::read_u32(&data[offset + 4..]) as usize;
                let length = LittleEndian::read_u32(&data[offset + 8..]) as usize;
                let allocate = LittleEndian::read_u32(&data[offset + 12..]) as usize;

                if allocate == 1 {
                    count += 1;
                }

                println!(
                    "offset:{:x} prev:{:x} next:{:x} length:{} allocated:{} {}",
                    offset + 16,
                    prev + 16,
                    next + 16,
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

            count
        } else {
            0
        }
    }

    fn initialize_data_account(&mut self) -> Account {
        let address = account_new();
        self.account_data.insert(
            address,
            AccountState {
                lamports: 0,
                data: vec![0; 4096],
                owner: Some(self.stack[0].id),
            },
        );

        address
    }
}

pub fn parse_and_resolve(src: &'static str, target: Target) -> ast::Namespace {
    let mut cache = FileResolver::default();

    cache.set_file_contents("test.sol", src.to_string());

    solang::parse_and_resolve(OsStr::new("test.sol"), &mut cache, target)
}

struct VmFunction<'a, 'b> {
    vm: &'a mut VirtualMachine,
    idx: usize,
    expected: u64,
    accounts: Vec<AccountMeta>,
    has_remaining: bool,
    arguments: Option<&'b [BorshToken]>,
    data_account: Option<usize>,
}

impl<'b> VmFunction<'_, 'b> {
    fn accounts(&mut self, accounts: Vec<(&str, Account)>) -> &mut Self {
        let accounts = accounts.into_iter().collect::<HashMap<&str, Account>>();
        let mut metas: Vec<AccountMeta> = Vec::new();

        for account in &self.vm.stack[0].idl.as_ref().unwrap().instructions[self.idx].accounts {
            match account {
                IdlAccountItem::IdlAccount(account) => {
                    if account.name == "dataAccount" {
                        self.data_account = Some(metas.len());
                    }

                    metas.push(AccountMeta {
                        pubkey: Pubkey(
                            accounts
                                .get(account.name.as_str())
                                .cloned()
                                .unwrap_or_else(|| panic!("account '{}' is missing", account.name)),
                        ),
                        is_writable: account.is_mut,
                        is_signer: account.is_signer,
                    });
                }
                IdlAccountItem::IdlAccounts(_) => unimplemented!("Solang does not use IdlAccounts"),
            }
        }

        assert_eq!(
            accounts.len(),
            metas.len(),
            "Number of accounts does not match IDL"
        );

        self.accounts = metas;
        self
    }

    fn remaining_accounts(&mut self, accounts: &[AccountMeta]) -> &mut Self {
        self.has_remaining = true;
        self.accounts.extend_from_slice(accounts);
        self
    }

    fn expected(&mut self, expected: u64) -> &mut Self {
        self.expected = expected;
        self
    }

    fn arguments(&mut self, args: &'b [BorshToken]) -> &mut Self {
        self.arguments = Some(args);
        self
    }

    fn call(&mut self) -> Option<BorshToken> {
        match self.call_with_error_code() {
            Ok(output) => output,
            Err(num) => panic!("unexpected return {num:#x}"),
        }
    }

    fn call_with_error_code(&mut self) -> Result<Option<BorshToken>, u64> {
        self.vm.return_data = None;
        let idl_instr = self.vm.stack[0].idl.as_ref().unwrap().instructions[self.idx].clone();
        let mut calldata = function_discriminator(&idl_instr.name);

        if !self.has_remaining {
            assert_eq!(
                idl_instr.accounts.len(),
                self.accounts.len(),
                "Incorrect number of accounts"
            );
        }

        println!(
            "function {} for {}",
            idl_instr.name,
            self.vm.stack[0].id.to_base58()
        );

        if let Some(args) = self.arguments {
            let mut encoded_data = encode_arguments(args);
            calldata.append(&mut encoded_data);
        }

        println!("input: {}", hex::encode(&calldata));

        let res = self.vm.execute(&self.accounts, &calldata);

        if let Some(idx) = self.data_account {
            self.vm
                .validate_account_data_heap(&self.accounts[idx].pubkey);
        }

        match res {
            ProgramResult::Ok(num) if num == self.expected => (),
            ProgramResult::Ok(num) => return Err(num),
            ProgramResult::Err(e) => panic!("error {e:?}"),
        }

        let return_data = if let Some((_, return_data)) = &self.vm.return_data {
            return_data.as_slice()
        } else {
            &[]
        };

        if let Some(ret) = &idl_instr.returns {
            let mut offset = 0;
            let decoded = decode_at_offset(
                return_data,
                &mut offset,
                ret,
                &self.vm.stack[0].idl.as_ref().unwrap().types,
            );
            assert_eq!(offset, return_data.len());
            Ok(Some(decoded))
        } else {
            assert_eq!(return_data.len(), 0);
            Ok(None)
        }
    }

    fn must_fail(&mut self) -> ProgramResult {
        self.vm.return_data = None;
        let idl_instr = self.vm.stack[0].idl.as_ref().unwrap().instructions[self.idx].clone();
        if !self.has_remaining {
            assert_eq!(
                idl_instr.accounts.len(),
                self.accounts.len(),
                "Incorrect number of accounts"
            );
        }

        let mut calldata = function_discriminator(&idl_instr.name);
        if let Some(args) = self.arguments {
            let mut encoded_data = encode_arguments(args);
            calldata.append(&mut encoded_data);
        }

        let result = self.vm.execute(&self.accounts, &calldata);

        if let Some(idx) = self.data_account {
            self.vm
                .validate_account_data_heap(&self.accounts[idx].pubkey);
        }

        if let ProgramResult::Ok(num) = result {
            assert_ne!(num, 0);
        }

        result
    }
}
