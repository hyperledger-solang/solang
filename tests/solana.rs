mod solana_helpers;

use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};
use ethabi::Token;
use libc::c_char;
use rand::Rng;
use sha2::{Digest, Sha256};
use solana_helpers::allocator_bump::Allocator;
use solana_rbpf::{
    error::EbpfError,
    memory_region::{AccessType, MemoryMapping, MemoryRegion},
    user_error::UserError,
    vm::{Config, DefaultInstructionMeter, EbpfVm, Executable, Syscall, SyscallObject},
};
use solang::{
    compile,
    file_cache::FileCache,
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

struct VirtualMachine {
    account_data: HashMap<Account, (Vec<u8>, Option<Account>)>,
    programs: Vec<Contract>,
    stack: Vec<Contract>,
    printbuf: String,
    output: Vec<u8>,
}

#[derive(Clone)]
struct Contract {
    program: Account,
    abi: ethabi::Contract,
    data: Account,
}

fn build_solidity(src: &str) -> VirtualMachine {
    let mut cache = FileCache::new();

    cache.set_file_contents("test.sol", src.to_string());

    let (res, ns) = compile(
        "test.sol",
        &mut cache,
        inkwell::OptimizationLevel::Default,
        Target::Solana,
        false,
    );

    diagnostics::print_messages(&mut cache, &ns, false);

    for v in &res {
        println!("contract size:{}", v.0.len());
    }

    assert_eq!(res.is_empty(), false);

    let mut account_data = HashMap::new();
    let mut programs = Vec::new();

    // resolve
    for (code, abi) in res {
        let program = account_new();

        account_data.insert(program, (code.clone(), None));

        let abi = ethabi::Contract::load(abi.as_bytes()).unwrap();

        let data = account_new();

        account_data.insert(data, ([0u8; 4096].to_vec(), Some(program)));

        programs.push(Contract { program, abi, data });
    }

    let cur = programs.last().unwrap().clone();

    VirtualMachine {
        account_data,
        programs,
        stack: vec![cur],
        printbuf: String::new(),
        output: Vec::new(),
    }
}

const MAX_PERMITTED_DATA_INCREASE: usize = 10 * 1024;

fn serialize_parameters(input: &[u8], vm: &VirtualMachine) -> Vec<u8> {
    let mut v: Vec<u8> = Vec::new();

    fn serialize_account(v: &mut Vec<u8>, key: &Account, acc: &(Vec<u8>, Option<Account>)) {
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
        v.write_all(&acc.1.unwrap_or([0u8; 32])).unwrap();
        // lamports
        v.write_u64::<LittleEndian>(0).unwrap();

        // account data
        v.write_u64::<LittleEndian>(acc.0.len() as u64).unwrap();
        v.write_all(&acc.0).unwrap();
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

    for (key, data) in &vm.account_data {
        serialize_account(&mut v, key, data);
    }

    // calldata
    v.write_u64::<LittleEndian>(input.len() as u64).unwrap();
    v.write_all(input).unwrap();

    // program id
    v.write_all(&[0u8; 32]).unwrap();

    v
}

// We want to extract the account data
fn deserialize_parameters(
    input: &[u8],
    accounts_data: &mut HashMap<Account, (Vec<u8>, Option<Account>)>,
) {
    let mut start = 0;

    let ka_num = LittleEndian::read_u64(&input[start..]);
    start += size_of::<u64>();

    for _ in 0..ka_num {
        start += 8;

        let account: Account = input[start..start + 32].try_into().unwrap();

        start += 32 + 32 + 8;

        let data_len = LittleEndian::read_u64(&input[start..]) as usize;
        start += size_of::<u64>();
        let data = input[start..start + data_len].to_vec();

        if let Some(entry) = accounts_data.get_mut(&account) {
            entry.0 = data;
        }

        start += data_len + MAX_PERMITTED_DATA_INCREASE;

        let padding = start % 8;
        if padding > 0 {
            start += 8 - padding
        }

        start += size_of::<u64>();
    }
}

struct Printer<'a> {
    context: Rc<RefCell<&'a mut VirtualMachine>>,
}

impl<'a> SyscallObject<UserError> for Printer<'a> {
    fn call(
        &mut self,
        vm_addr: u64,
        len: u64,
        _arg3: u64,
        _arg4: u64,
        _arg5: u64,
        memory_mapping: &MemoryMapping,
    ) -> Result<u64, EbpfError<UserError>> {
        let host_addr = memory_mapping.map(AccessType::Load, vm_addr, len)?;
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
                context.printbuf.push_str(message);
            }
            Ok(0)
        }
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
    ) -> Result<u64, EbpfError<UserError>> {
        let arrays = translate_slice::<(u64, u64)>(memory_mapping, src, len)?;

        let mut hasher = Sha256::new();
        for (addr, len) in arrays {
            let buf = translate_slice::<u8>(memory_mapping, *addr, *len)?;
            println!("hashing: {}", hex::encode(buf));
            hasher.update(buf);
        }

        let hash = hasher.finalize();

        let hash_result = translate_slice_mut::<u8>(memory_mapping, dest, hash.len() as u64)?;

        hash_result.copy_from_slice(&hash);

        println!("sol_sha256: {}", hex::encode(hash));

        Ok(0)
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
    ) -> Result<u64, EbpfError<UserError>> {
        let arrays = translate_slice::<(u64, u64)>(memory_mapping, src, len)?;

        let mut hasher = Keccak::v256();
        let mut hash = [0u8; 32];
        for (addr, len) in arrays {
            let buf = translate_slice::<u8>(memory_mapping, *addr, *len)?;
            println!("hashing: {}", hex::encode(buf));
            hasher.update(buf);
        }
        hasher.finalize(&mut hash);

        let hash_result = translate_slice_mut::<u8>(memory_mapping, dest, hash.len() as u64)?;

        hash_result.copy_from_slice(&hash);

        println!("sol_keccak256: {}", hex::encode(hash));

        Ok(0)
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
pub const MM_HEAP_START: u64 = 0x300000000;
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
    ) -> Result<u64, EbpfError<UserError>> {
        let align = align_of::<u128>();
        let layout = match Layout::from_size_align(size as usize, align) {
            Ok(layout) => layout,
            Err(_) => return Ok(0),
        };
        if free_addr == 0 {
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

/// Rust representation of C's SolAccountInfo
#[derive(Debug)]
struct SolAccountInfo {
    key_addr: u64,
    lamports_addr: u64,
    data_len: u64,
    data_addr: u64,
    owner_addr: u64,
    rent_epoch: u64,
    is_signer: bool,
    is_writable: bool,
    executable: bool,
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
    calldata: &'a [u8],
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

impl<'a> SyscallObject<UserError> for SyscallInvokeSignedC<'a> {
    fn call(
        &mut self,
        instruction_addr: u64,
        _account_infos_addr: u64,
        _account_infos_len: u64,
        _signers_seeds_addr: u64,
        _signers_seeds_len: u64,
        memory_mapping: &MemoryMapping,
    ) -> Result<u64, EbpfError<UserError>> {
        let instruction = self
            .translate_instruction(instruction_addr, memory_mapping)
            .expect("instruction not valid");

        let data_id: Account = instruction.data[..32].try_into().unwrap();

        println!("instruction:{:?}", instruction);

        if let Ok(mut context) = self.context.try_borrow_mut() {
            let p = context
                .programs
                .iter()
                .find(|p| p.program == instruction.program_id.0 && p.data == data_id)
                .unwrap()
                .clone();

            context.stack.insert(0, p);

            context.execute(&instruction.data);

            let parameter_bytes = serialize_parameters(&self.calldata, &context);

            assert_eq!(parameter_bytes.len(), self.input.len());

            unsafe {
                std::ptr::copy(
                    parameter_bytes.as_ptr(),
                    self.input.as_ptr() as *mut u8,
                    parameter_bytes.len(),
                );
            }

            context.stack.remove(0);
        }

        Ok(0)
    }
}

impl VirtualMachine {
    fn execute(&mut self, calldata: &[u8]) {
        println!("running bpf with calldata:{}", hex::encode(calldata));

        let parameter_bytes = serialize_parameters(&calldata, &self);
        let heap = vec![0_u8; DEFAULT_HEAP_SIZE];
        let heap_region = MemoryRegion::new_from_slice(&heap, MM_HEAP_START, true);

        let program = &self.stack[0];

        let executable =
            Executable::<UserError>::from_elf(&self.account_data[&program.program].0, None)
                .expect("should work");
        let mut vm = EbpfVm::<UserError, DefaultInstructionMeter>::new(
            executable.as_ref(),
            Config::default(),
            &parameter_bytes,
            &[heap_region],
        )
        .unwrap();

        let context = Rc::new(RefCell::new(self));

        vm.register_syscall_ex(
            "sol_log_",
            Syscall::Object(Box::new(Printer {
                context: context.clone(),
            })),
        )
        .unwrap();

        vm.register_syscall_ex("sol_sha256", Syscall::Object(Box::new(SolSha256())))
            .unwrap();

        vm.register_syscall_ex("sol_keccak256", Syscall::Object(Box::new(SolKeccak256())))
            .unwrap();

        vm.register_syscall_ex(
            "sol_invoke_signed_c",
            Syscall::Object(Box::new(SyscallInvokeSignedC {
                context: context.clone(),
                input: &parameter_bytes,
                calldata: &calldata,
            })),
        )
        .unwrap();
        vm.register_syscall_ex(
            "sol_alloc_free_",
            Syscall::Object(Box::new(SyscallAllocFree {
                allocator: Allocator::new(heap, MM_HEAP_START),
            })),
        )
        .unwrap();

        let res = vm
            .execute_program_interpreted(&mut DefaultInstructionMeter {})
            .unwrap();

        let mut elf = context.try_borrow_mut().unwrap();

        deserialize_parameters(&parameter_bytes, &mut elf.account_data);

        let output = &elf.account_data[&elf.stack[0].data].0;

        VirtualMachine::validate_heap(&output);

        let len = LittleEndian::read_u32(&output[4..]) as usize;
        let offset = LittleEndian::read_u32(&output[8..]) as usize;
        elf.output = output[offset..offset + len].to_vec();

        println!("return: {}", hex::encode(&elf.output));

        assert_eq!(res, 0);
    }

    fn constructor(&mut self, args: &[Token]) {
        let program = &self.stack[0];

        println!("constructor for {}", hex::encode(&program.data));

        let calldata = if let Some(constructor) = &program.abi.constructor {
            constructor
                .encode_input(program.data.to_vec(), args)
                .unwrap()
        } else {
            program.data.to_vec()
        };

        self.execute(&calldata);
    }

    fn function(&mut self, name: &str, args: &[Token]) -> Vec<Token> {
        let program = &self.stack[0];

        println!("function for {}", hex::encode(&program.data));

        let mut calldata: Vec<u8> = program.data.to_vec();

        match program.abi.functions[name][0].encode_input(args) {
            Ok(n) => calldata.extend(&n),
            Err(x) => panic!("{}", x),
        };

        println!("input: {}", hex::encode(&calldata));

        self.execute(&calldata);

        println!("output: {}", hex::encode(&self.output));

        let program = &self.stack[0];

        program.abi.functions[name][0]
            .decode_output(&self.output)
            .unwrap()
    }

    fn data(&self) -> &Vec<u8> {
        let program = &self.stack[0];

        &self.account_data[&program.data].0
    }

    fn set_program(&mut self, no: usize) {
        let cur = self.programs[no].clone();

        self.stack = vec![cur];
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
}

pub fn parse_and_resolve(src: &'static str, target: Target) -> ast::Namespace {
    let mut cache = FileCache::new();

    cache.set_file_contents("test.sol", src.to_string());

    solang::parse_and_resolve("test.sol", &mut cache, target)
}

pub fn first_error(errors: Vec<ast::Diagnostic>) -> String {
    match errors.iter().find(|m| m.level == ast::Level::Error) {
        Some(m) => m.message.to_owned(),
        None => panic!("no errors found"),
    }
}
