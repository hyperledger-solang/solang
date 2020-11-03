//
extern crate byteorder;
extern crate ethabi;
extern crate ethereum_types;
extern crate libc;
extern crate solana_rbpf;
extern crate solang;

mod solana_helpers;

use byteorder::{LittleEndian, WriteBytesExt};
use ethabi::Token;
use libc::c_char;
use solana_helpers::allocator_bump::BPFAllocator;
use solana_rbpf::{
    error::EbpfError,
    memory_region::{translate_addr, MemoryRegion},
    user_error::UserError,
    vm::{Config, EbpfVm, SyscallObject},
};
use solang::{compile, file_cache::FileCache, sema::diagnostics, Target};
use std::alloc::Layout;
use std::io::Write;
use std::mem::align_of;

fn build_solidity(src: &'static str) -> VM {
    let mut cache = FileCache::new();

    cache.set_file_contents("test.sol".to_string(), src.to_string());

    let (res, ns) = compile(
        "test.sol",
        &mut cache,
        inkwell::OptimizationLevel::Default,
        Target::Solana,
    );

    diagnostics::print_messages(&mut cache, &ns, false);

    for v in &res {
        println!("contract size:{}", v.0.len());
    }

    assert_eq!(res.is_empty(), false);

    // resolve
    let (code, abi) = res.last().unwrap().clone();

    VM {
        code,
        abi: ethabi::Contract::load(abi.as_bytes()).unwrap(),
        printbuf: String::new(),
    }
}

const MAX_PERMITTED_DATA_INCREASE: usize = 10 * 1024;

fn serialize_parameters(input: &[u8]) -> Vec<u8> {
    let mut v: Vec<u8> = Vec::new();

    // ka_num
    v.write_u64::<LittleEndian>(1).unwrap();
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
    v.write_all(&[0u8; 32]).unwrap();
    // owner
    v.write_all(&[0u8; 32]).unwrap();
    // lamports
    v.write_u64::<LittleEndian>(0).unwrap();

    // account data
    // data len
    v.write_u64::<LittleEndian>(0).unwrap();
    v.write_all(&[0u8; MAX_PERMITTED_DATA_INCREASE]).unwrap();

    let padding = v.len() % 8;
    if padding != 0 {
        let mut p = Vec::new();
        p.resize(8 - padding, 0);
        v.extend_from_slice(&p);
    }
    // rent epoch
    v.write_u64::<LittleEndian>(0).unwrap();

    // calldata
    v.write_u64::<LittleEndian>(input.len() as u64).unwrap();
    v.write_all(input).unwrap();

    // program id
    v.write_all(&[0u8; 32]).unwrap();

    v
}

struct VM {
    code: Vec<u8>,
    abi: ethabi::Contract,
    printbuf: String,
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
        ro_regions: &[MemoryRegion],
        _rw_regions: &[MemoryRegion],
    ) -> Result<u64, EbpfError<UserError>> {
        let host_addr = translate_addr(vm_addr, len as usize, "Load", 0, ro_regions)?;
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
    allocator: BPFAllocator,
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
        _ro_regions: &[MemoryRegion],
        _rw_regions: &[MemoryRegion],
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

impl VM {
    fn execute(&self, buf: &mut String, calldata: &[u8]) {
        println!("running bpf with calldata:{}", hex::encode(calldata));

        let executable =
            EbpfVm::<UserError>::create_executable_from_elf(&self.code, None).expect("should work");
        let mut vm = EbpfVm::<UserError>::new(executable.as_ref(), Config::default()).unwrap();

        vm.register_syscall_with_context_ex("sol_log_", Box::new(Printer { buf }))
            .unwrap();

        let heap = vec![0_u8; DEFAULT_HEAP_SIZE];
        let heap_region = MemoryRegion::new_from_slice(&heap, MM_HEAP_START);
        vm.register_syscall_with_context_ex(
            "sol_alloc_free_",
            Box::new(SyscallAllocFree {
                allocator: BPFAllocator::new(heap, MM_HEAP_START),
            }),
        )
        .unwrap();

        let parameter_bytes = serialize_parameters(&calldata);

        let res = vm
            .execute_program(&parameter_bytes, &[], &[heap_region])
            .unwrap();

        assert_eq!(res, 0);
    }

    fn constructor(&mut self, args: &[Token]) {
        let calldata = if let Some(constructor) = &self.abi.constructor {
            constructor.encode_input(Vec::new(), args).unwrap()
        } else {
            Vec::new()
        };

        let mut buf = String::new();
        self.execute(&mut buf, &calldata);
        self.printbuf = buf;
    }

    fn function(&mut self, name: &str, args: &[Token]) {
        let calldata = match self.abi.functions[name][0].encode_input(args) {
            Ok(n) => n,
            Err(x) => panic!(format!("{}", x)),
        };

        let mut buf = String::new();
        self.execute(&mut buf, &calldata);
        self.printbuf = buf;
    }
}

#[test]
fn simple() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            constructor() {
                print("Hello from constructor");
            }

            function test() public {
                print("Hello from function");
            }
        }"#,
    );

    vm.constructor(&[]);

    assert_eq!(vm.printbuf, "Hello from constructor");

    vm.printbuf = String::new();

    vm.function("test", &[]);

    assert_eq!(vm.printbuf, "Hello from function");
}

#[test]
fn basic() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            function test(uint32 x, uint64 y) public {
                if (x == 10) {
                    print("x is 10");
                }
                if (y == 102) {
                    print("y is 102");
                }
            }
        }"#,
    );

    vm.function(
        "test",
        &[
            ethabi::Token::Uint(ethereum_types::U256::from(10)),
            ethabi::Token::Uint(ethereum_types::U256::from(10)),
        ],
    );

    assert_eq!(vm.printbuf, "x is 10");

    vm.function(
        "test",
        &[
            ethabi::Token::Uint(ethereum_types::U256::from(99)),
            ethabi::Token::Uint(ethereum_types::U256::from(102)),
        ],
    );

    assert_eq!(vm.printbuf, "y is 102");
}
