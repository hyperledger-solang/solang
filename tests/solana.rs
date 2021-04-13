mod solana_helpers;

use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};
use ethabi::Token;
use libc::c_char;
use rand::Rng;
use solana_helpers::allocator_bump::Allocator;
use solana_rbpf::{
    error::EbpfError,
    memory_region::{AccessType, MemoryMapping, MemoryRegion},
    user_error::UserError,
    vm::{Config, DefaultInstructionMeter, EbpfVm, Executable, Syscall, SyscallObject},
};
use solang::{compile, file_cache::FileCache, sema::diagnostics, Target};
use std::alloc::Layout;
use std::io::Write;
use std::mem::{align_of, size_of};

mod solana_tests;

type Account = [u8; 32];

fn account_new() -> Account {
    let mut rng = rand::thread_rng();

    let mut a = [0u8; 32];

    rng.fill(&mut a[..]);

    a
}

fn build_solidity(src: &str) -> Program {
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

    // resolve
    let (code, abi) = res.last().unwrap().clone();

    Program {
        code,
        abi: ethabi::Contract::load(abi.as_bytes()).unwrap(),
        account: account_new(),
        printbuf: String::new(),
        output: Vec::new(),
        data: Vec::new(),
    }
}

const MAX_PERMITTED_DATA_INCREASE: usize = 10 * 1024;

fn serialize_parameters(input: &[u8], account: &Account, data: &[u8]) -> Vec<u8> {
    let mut v: Vec<u8> = Vec::new();

    // ka_num
    v.write_u64::<LittleEndian>(2).unwrap();
    for account_no in 0..2 {
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
        if account_no == 1 {
            v.write_all(&account[..]).unwrap();
        } else {
            v.write_all(&account_new()).unwrap();
        }
        // owner
        v.write_all(&[0u8; 32]).unwrap();
        // lamports
        v.write_u64::<LittleEndian>(0).unwrap();

        // account data
        // data len
        if account_no == 1 {
            v.write_u64::<LittleEndian>(4096).unwrap();
            let mut data = data.to_vec();
            data.resize(4096, 0);
            v.write_all(&data).unwrap();
        } else {
            v.write_u64::<LittleEndian>(4096).unwrap();
            v.write_all(&[0u8; 4096]).unwrap();
        }
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

    // calldata
    v.write_u64::<LittleEndian>(input.len() as u64).unwrap();
    v.write_all(input).unwrap();

    // program id
    v.write_all(&[0u8; 32]).unwrap();

    v
}

// We want to extract the account data
fn deserialize_parameters(input: &[u8]) -> Vec<Vec<u8>> {
    let mut start = 0;

    let ka_num = LittleEndian::read_u64(&input[start..]);
    start += size_of::<u64>();

    let mut res = Vec::new();

    for _ in 0..ka_num {
        start += 8 + 32 + 32 + 8;

        let data_len = LittleEndian::read_u64(&input[start..]) as usize;
        start += size_of::<u64>();

        res.push(input[start..start + data_len].to_vec());

        start += data_len + MAX_PERMITTED_DATA_INCREASE;

        let padding = start % 8;
        if padding > 0 {
            start += 8 - padding
        }

        start += size_of::<u64>();
    }

    res
}

struct Program {
    code: Vec<u8>,
    abi: ethabi::Contract,
    account: Account,
    printbuf: String,
    data: Vec<u8>,
    output: Vec<u8>,
}

struct Printer<'a> {
    buf: &'a mut String,
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
            self.buf.push_str(message);
            Ok(0)
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

impl Program {
    fn execute(&mut self, buf: &mut String, calldata: &[u8]) {
        println!("running bpf with calldata:{}", hex::encode(calldata));

        let parameter_bytes = serialize_parameters(&calldata, &self.account, &self.data);
        let heap = vec![0_u8; DEFAULT_HEAP_SIZE];
        let heap_region = MemoryRegion::new_from_slice(&heap, MM_HEAP_START, true);

        let executable = Executable::<UserError>::from_elf(&self.code, None).expect("should work");
        let mut vm = EbpfVm::<UserError, DefaultInstructionMeter>::new(
            executable.as_ref(),
            Config::default(),
            &parameter_bytes,
            &[heap_region],
        )
        .unwrap();

        vm.register_syscall_ex("sol_log_", Syscall::Object(Box::new(Printer { buf })))
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

        let mut accounts = deserialize_parameters(&parameter_bytes);

        let output = accounts.remove(0);
        let data = accounts.remove(0);

        let len = LittleEndian::read_u64(&output);
        self.output = output[8..len as usize + 8].to_vec();
        self.data = data;

        println!("account: {}", hex::encode(&self.output));

        assert_eq!(res, 0);
    }

    fn constructor(&mut self, args: &[Token]) {
        let calldata = if let Some(constructor) = &self.abi.constructor {
            constructor
                .encode_input(self.account.to_vec(), args)
                .unwrap()
        } else {
            self.account.to_vec()
        };

        let mut buf = String::new();
        self.execute(&mut buf, &calldata);
        self.printbuf = buf;
    }

    fn function(&mut self, name: &str, args: &[Token]) -> Vec<Token> {
        let mut calldata: Vec<u8> = self.account.to_vec();

        match self.abi.functions[name][0].encode_input(args) {
            Ok(n) => calldata.extend(&n),
            Err(x) => panic!("{}", x),
        };

        println!("input: {}", hex::encode(&calldata));

        let mut buf = String::new();
        self.execute(&mut buf, &calldata);
        self.printbuf = buf;

        println!("output: {}", hex::encode(&self.output));

        self.abi.functions[name][0]
            .decode_output(&self.output)
            .unwrap()
    }
}
