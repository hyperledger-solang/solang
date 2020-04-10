extern crate ethabi;
extern crate ethereum_types;
extern crate num_derive;
extern crate num_traits;
extern crate solang;
extern crate wasmi;

use ethabi::Token;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use std::collections::HashMap;
use std::fmt;
use wasmi::memory_units::Pages;
use wasmi::*;

use solang::output;
use solang::{compile, Target};

struct ContractStorage {
    memory: MemoryRef,
    input: Vec<u8>,
    output: Vec<u8>,
    store: HashMap<[u8; 32], [u8; 32]>,
}

#[derive(FromPrimitive)]
#[allow(non_camel_case_types)]
pub enum Extern {
    getCallDataSize = 1,
    callDataCopy,
    storageLoad,
    storageStore,
    finish,
    revert,
    printMem,
}

impl ContractStorage {
    fn new() -> Self {
        ContractStorage {
            memory: MemoryInstance::alloc(Pages(2), Some(Pages(2))).unwrap(),
            store: HashMap::new(),
            input: Vec::new(),
            output: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct HostCodeFinish {}

impl HostError for HostCodeFinish {}

impl fmt::Display for HostCodeFinish {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "finish")
    }
}

#[derive(Debug, Clone, PartialEq)]
struct HostCodeRevert {}

impl fmt::Display for HostCodeRevert {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "revert")
    }
}

impl HostError for HostCodeRevert {}

impl Externals for ContractStorage {
    fn invoke_index(
        &mut self,
        index: usize,
        args: RuntimeArgs,
    ) -> Result<Option<RuntimeValue>, Trap> {
        match FromPrimitive::from_usize(index) {
            Some(Extern::getCallDataSize) => Ok(Some(RuntimeValue::I32(self.input.len() as i32))),
            Some(Extern::callDataCopy) => {
                let dest = args.nth_checked::<u32>(0)?;
                let input_offset = args.nth_checked::<u32>(1)? as usize;
                let input_len = args.nth_checked::<u32>(2)? as usize;

                self.memory
                    .set(
                        dest,
                        &self.input[input_offset as usize..input_offset + input_len],
                    )
                    .expect("calldatacopy should work");

                Ok(None)
            }
            Some(Extern::finish) => {
                let src: u32 = args.nth_checked(0)?;
                let len: u32 = args.nth_checked(1)?;

                self.output.resize(len as usize, 0);

                self.memory.get_into(src, &mut self.output).unwrap();

                println!("finish: {} {}", len, self.output.len());

                Err(Trap::new(TrapKind::Host(Box::new(HostCodeFinish {}))))
            }
            Some(Extern::revert) => {
                let src: u32 = args.nth_checked(0)?;
                let len: u32 = args.nth_checked(1)?;

                self.output.resize(len as usize, 0);

                self.memory.get_into(src, &mut self.output).unwrap();

                Err(Trap::new(TrapKind::Host(Box::new(HostCodeRevert {}))))
            }
            Some(Extern::storageLoad) => {
                let key_ptr: u32 = args.nth_checked(0)?;
                let data_ptr: u32 = args.nth_checked(1)?;

                let mut key = [0u8; 32];

                self.memory
                    .get_into(key_ptr, &mut key)
                    .expect("copy key from wasm memory");

                let res = if let Some(v) = self.store.get(&key) {
                    v
                } else {
                    &[0u8; 32]
                };
                self.memory
                    .set(data_ptr, res)
                    .expect("copy key from wasm memory");

                Ok(None)
            }
            Some(Extern::storageStore) => {
                let key_ptr: u32 = args.nth_checked(0)?;
                let data_ptr: u32 = args.nth_checked(1)?;

                let mut key = [0u8; 32];
                let mut data = [0u8; 32];

                self.memory
                    .get_into(key_ptr, &mut key)
                    .expect("copy key from wasm memory");

                self.memory
                    .get_into(data_ptr, &mut data)
                    .expect("copy key from wasm memory");

                if data.iter().any(|n| *n != 0) {
                    self.store.insert(key, data);
                } else {
                    self.store.remove(&key);
                }
                Ok(None)
            }
            Some(Extern::printMem) => {
                let data_ptr: u32 = args.nth_checked(0)?;
                let len: u32 = args.nth_checked(1)?;

                let mut buf = Vec::new();
                buf.resize(len as usize, 0u8);

                if let Err(e) = self.memory.get_into(data_ptr, &mut buf) {
                    panic!("ext_print: {}", e);
                }

                println!("{}", String::from_utf8_lossy(&buf));

                Ok(None)
            }
            _ => panic!("external {} unknown", index),
        }
    }
}

impl ModuleImportResolver for ContractStorage {
    fn resolve_func(&self, field_name: &str, signature: &Signature) -> Result<FuncRef, Error> {
        let index = match field_name {
            "getCallDataSize" => Extern::getCallDataSize,
            "callDataCopy" => Extern::callDataCopy,
            "finish" => Extern::finish,
            "revert" => Extern::revert,
            "storageStore" => Extern::storageStore,
            "storageLoad" => Extern::storageLoad,
            "printMem" => Extern::printMem,
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
        Ok(self.memory.clone())
    }
}

struct TestRuntime {
    module: ModuleRef,
    abi: ethabi::Contract,
}

impl TestRuntime {
    fn function(&self, store: &mut ContractStorage, name: &str, args: &[Token]) -> Vec<Token> {
        let calldata = match self.abi.functions[name][0].encode_input(args) {
            Ok(n) => n,
            Err(x) => panic!(format!("{}", x)),
        };

        println!("FUNCTION CALLDATA: {}", hex::encode(&calldata));

        store.input = calldata;

        match self.module.invoke_export("main", &[], store) {
            Err(wasmi::Error::Trap(trap)) => match trap.kind() {
                TrapKind::Host(_) => {}
                _ => panic!("fail to invoke main: {}", trap),
            },
            Ok(_) => {}
            Err(e) => panic!("fail to invoke main: {}", e),
        }

        println!("RETURNDATA: {}", hex::encode(&store.output));

        self.abi.functions[name][0]
            .decode_output(&store.output)
            .unwrap()
    }

    fn constructor(&mut self, store: &mut ContractStorage, args: &[Token]) {
        let calldata = if let Some(constructor) = &self.abi.constructor {
            constructor.encode_input(Vec::new(), args).unwrap()
        } else {
            Vec::new()
        };

        println!("CONSTRUCTOR CALLDATA: {}", hex::encode(&calldata));

        store.input = calldata;

        match self.module.invoke_export("main", &[], store) {
            Err(wasmi::Error::Trap(trap)) => match trap.kind() {
                TrapKind::Host(_) => {}
                _ => panic!("fail to invoke main: {}", trap),
            },
            Ok(_) => {}
            Err(e) => panic!("fail to invoke main: {}", e),
        }

        println!("DEPLOYER RETURNS: {}", hex::encode(&store.output));

        let module = Module::from_buffer(&store.output).expect("parse wasm should work");

        self.module = ModuleInstance::new(
            &module,
            &ImportsBuilder::new().with_resolver("ethereum", store),
        )
        .expect("Failed to instantiate module")
        .run_start(&mut NopExternals)
        .expect("Failed to run start function in module");
    }
}

fn build_solidity(src: &'static str) -> (TestRuntime, ContractStorage) {
    let (mut res, errors) = compile(
        src,
        "test.sol",
        inkwell::OptimizationLevel::Default,
        Target::Ewasm,
    );

    output::print_messages("test.sol", src, &errors, false);

    assert_eq!(res.len(), 1);

    // resolve
    let (bc, abi) = res.pop().unwrap();

    let module = Module::from_buffer(bc).expect("parse wasm should work");

    let store = ContractStorage::new();

    (
        TestRuntime {
            module: ModuleInstance::new(
                &module,
                &ImportsBuilder::new().with_resolver("ethereum", &store),
            )
            .expect("Failed to instantiate module")
            .run_start(&mut NopExternals)
            .expect("Failed to run start function in module"),
            abi: ethabi::Contract::load(abi.as_bytes()).unwrap(),
        },
        store,
    )
}

#[test]
fn simple_solidiy_compile_and_run() {
    // parse
    let (mut runtime, mut store) = build_solidity(
        "
        contract test {
            function foo() public returns (uint32) {
                return 2;
            }
        }",
    );

    // call constructor
    runtime.constructor(&mut store, &[]);

    let returns = runtime.function(&mut store, "foo", &[]);

    assert_eq!(
        returns,
        vec![ethabi::Token::Uint(ethereum_types::U256::from(2))]
    );
}

#[test]
fn simple_loops() {
    let (mut runtime, mut store) = build_solidity(
        r##"
contract test3 {
	function foo(uint32 a) public returns (uint32) {
		uint32 b = 50 - a;
		uint32 c;
		c = 100 * b;
		c += 5;
		return a * 1000 + c;
	}

	function bar(uint32 b, bool x) public returns (uint32) {
		uint32 i = 1;
		if (x) {
			do {
				i += 10;
			}
			while (b-- > 0);
		} else {
			uint32 j;
			for (j=2; j<10; j++) {
				i *= 3;
			}
		}
		return i;
	}

	function baz(uint32 x) public returns (uint32) {
		for (uint32 i = 0; i<100; i++) {
			x *= 7;

			if (x > 200) {
				break;
			}

			x++;
		}

		return x;
	}
}"##,
    );

    // call constructor
    runtime.constructor(&mut store, &[]);

    for i in 0..=50 {
        let res = ((50 - i) * 100 + 5) + i * 1000;

        let returns = runtime.function(
            &mut store,
            "foo",
            &[ethabi::Token::Uint(ethereum_types::U256::from(i))],
        );

        assert_eq!(
            returns,
            vec![ethabi::Token::Uint(ethereum_types::U256::from(res))]
        );
    }

    for i in 0..=50 {
        let res = (i + 1) * 10 + 1;

        let returns = runtime.function(
            &mut store,
            "bar",
            &[
                ethabi::Token::Uint(ethereum_types::U256::from(i)),
                ethabi::Token::Bool(true),
            ],
        );

        assert_eq!(
            returns,
            vec![ethabi::Token::Uint(ethereum_types::U256::from(res))]
        );
    }

    for i in 0..=50 {
        let mut res = 1;

        for _ in 2..10 {
            res *= 3;
        }

        let returns = runtime.function(
            &mut store,
            "bar",
            &[
                ethabi::Token::Uint(ethereum_types::U256::from(i)),
                ethabi::Token::Bool(false),
            ],
        );

        assert_eq!(
            returns,
            vec![ethabi::Token::Uint(ethereum_types::U256::from(res))]
        );
    }

    for i in 1..=50 {
        let mut res = i;

        for _ in 0..100 {
            res *= 7;
            if res > 200 {
                break;
            }
            res += 1;
        }

        let returns = runtime.function(
            &mut store,
            "baz",
            &[ethabi::Token::Uint(ethereum_types::U256::from(i))],
        );

        assert_eq!(
            returns,
            vec![ethabi::Token::Uint(ethereum_types::U256::from(res))]
        );
    }
}

#[test]
fn stack_test() {
    let (mut runtime, mut store) = build_solidity(
        r##"
contract test3 {
	function foo() public returns (bool) {
		uint b = 18446744073709551616;
    uint c = 36893488147419103232;

    return b * 2 == c;
	}
}"##,
    );

    // call constructor
    runtime.constructor(&mut store, &[]);

    let returns = runtime.function(&mut store, "foo", &[]);

    assert_eq!(returns, vec![ethabi::Token::Bool(true)]);
}

#[test]
fn abi_call_return_test() {
    let (mut runtime, mut store) = build_solidity(
        r##"
contract test {
	function foo() public returns (uint32) {
    return 102;
	}
}"##,
    );

    // call constructor
    runtime.constructor(&mut store, &[]);

    let returns = runtime.function(&mut store, "foo", &[]);

    assert_eq!(
        returns,
        vec![ethabi::Token::Uint(ethereum_types::U256::from(102))]
    );
}

#[test]
fn abi_call_pass_return_test() {
    let (mut runtime, mut store) = build_solidity(
        r##"
contract test {
	function foo(uint32 a) public returns (uint32) {
    return a;
	}
}"##,
    );

    // call constructor
    runtime.constructor(&mut store, &[]);

    for val in [102i32, 255, 256, 0x7fff_ffff].iter() {
        let returns = runtime.function(
            &mut store,
            "foo",
            &[ethabi::Token::Uint(ethereum_types::U256::from(*val))],
        );

        assert_eq!(
            returns,
            vec![ethabi::Token::Uint(ethereum_types::U256::from(*val))]
        );
    }
}

#[test]
fn contract_storage_test() {
    let (mut runtime, mut store) = build_solidity(
        r##"
contract test {
uint32 foo;
constructor() public {
    foo = 102;
}
	function getFoo() public returns (uint32) {
    return foo + 256;
	}
	function setFoo(uint32 a) public  {
    foo = a - 256;
	}
}"##,
    );

    // call constructor
    runtime.constructor(&mut store, &[]);

    for val in [4096u32, 1000u32].iter() {
        let eval = ethabi::Token::Uint(ethereum_types::U256::from(*val));
        // create call for foo
        let returns = runtime.function(&mut store, "setFoo", &[eval]);

        assert_eq!(returns, vec![]);

        // create call for foo
        let returns = runtime.function(&mut store, "getFoo", &[]);

        let eval = ethabi::Token::Uint(ethereum_types::U256::from(*val));
        assert_eq!(returns, vec![eval]);
    }
}

#[test]
fn large_ints_encoded() {
    let (mut runtime, mut store) = build_solidity(
        r##"
    contract test {
        uint foo;
        constructor() public {
            foo = 102;
        }
        function getFoo() public returns (uint) {
            return foo + 256;
        }
        function setFoo(uint a) public  {
            foo = a - 256;
        }
}"##,
    );

    // call constructor
    runtime.constructor(&mut store, &[]);

    for val in [4096u32, 1000u32].iter() {
        let eval = ethabi::Token::Uint(ethereum_types::U256::from(*val));
        // create call for foo
        let returns = runtime.function(&mut store, "setFoo", &[eval]);

        assert_eq!(returns, vec![]);

        // create call for foo
        let returns = runtime.function(&mut store, "getFoo", &[]);

        let eval = ethabi::Token::Uint(ethereum_types::U256::from(*val));
        assert_eq!(returns, vec![eval]);
    }
}

#[test]
fn address() {
    let (mut runtime, mut store) = build_solidity(
        "
        contract address_tester {
            function encode_const() public returns (address) {
                return 0x52908400098527886E0F7030069857D2E4169EE7;
            }

            function test_arg(address foo) public {
                assert(foo == 0x27b1fdb04752bbc536007a920d24acb045561c26);

                // this literal is a number
                int x = 0x27b1fdb047_52bbc536007a920d24acb045561C26;
                assert(int(foo) == x);
            }

            function allones() public returns (address) {
                return address(1);
            }
        }",
    );

    // call constructor
    runtime.constructor(&mut store, &[]);

    let ret = runtime.function(&mut store, "encode_const", &[]);

    assert_eq!(
        ret,
        [ethabi::Token::Address(ethereum_types::Address::from_slice(
            &hex::decode("52908400098527886E0F7030069857D2E4169EE7").unwrap()
        ))]
    );

    runtime.function(
        &mut store,
        "test_arg",
        &[ethabi::Token::Address(ethereum_types::Address::from_slice(
            &hex::decode("27b1fdb04752bbc536007a920d24acb045561c26").unwrap(),
        ))],
    );

    let ret = runtime.function(&mut store, "allones", &[]);

    assert_eq!(
        ret,
        [ethabi::Token::Address(ethereum_types::Address::from_slice(
            &hex::decode("0000000000000000000000000000000000000001").unwrap()
        ))]
    );

    // no arithmetic/bitwise allowed on address
    // no ordered comparison allowed
    // address 0x27b1fdb04752bbc536007a920d24acb045561C26 should be a warning
}

#[test]
fn bytes() {
    let (mut runtime, mut store) = build_solidity(
        r##"
        contract bar {
            bytes4 constant foo = hex"11223344";

            function get_foo() public returns (bytes4) {
                return foo;
            }

            function bytes4asuint32() public view returns (uint32) {
                return uint32(foo);
            }

            function bytes4asuint64() public view returns (uint64) {
                return uint64(bytes8(foo));
            }

            function bytes4asbytes2() public view returns (bytes2) {
                return bytes2(foo);
            }

            function passthrough(bytes4 bar) public view returns (bytes4) {
                return bar;
            }

            function entry(uint index) public view returns (bytes1) {
                return foo[index];
            }

            function entry2(uint index) public pure returns (bytes1) {
                return hex"AABBCCDD"[index];
            }

            function shiftedleft() public view returns (bytes4) {
                return foo << 8;
            }

            function shiftedright() public view returns (bytes4) {
                return foo >> 8;
            }
        }"##,
    );

    runtime.constructor(&mut store, &[]);

    let ret = runtime.function(&mut store, "get_foo", &[]);

    assert_eq!(
        ret,
        [ethabi::Token::FixedBytes(vec!(0x11, 0x22, 0x33, 0x44))]
    );

    let ret = runtime.function(&mut store, "bytes4asuint32", &[]);

    assert_eq!(
        ret,
        [ethabi::Token::Uint(ethereum_types::U256::from(
            0x11_22_33_44
        ))]
    );

    let ret = runtime.function(&mut store, "bytes4asuint64", &[]);

    assert_eq!(
        ret,
        [ethabi::Token::Uint(ethereum_types::U256::from(
            0x1122_3344_0000_0000u64
        ))]
    );

    let ret = runtime.function(&mut store, "bytes4asbytes2", &[]);

    assert_eq!(ret, [ethabi::Token::FixedBytes(vec!(0x11, 0x22))]);

    let val = vec![ethabi::Token::FixedBytes(vec![0x41, 0x42, 0x43, 0x44])];

    assert_eq!(runtime.function(&mut store, "passthrough", &val), val);

    let val = vec![ethabi::Token::Uint(ethereum_types::U256::from(1))];

    let ret = runtime.function(&mut store, "entry", &val);

    assert_eq!(ret, [ethabi::Token::FixedBytes(vec!(0x22))]);

    let ret = runtime.function(&mut store, "entry2", &val);

    assert_eq!(ret, [ethabi::Token::FixedBytes(vec!(0xBB))]);
}

#[test]
fn array() {
    let (mut runtime, mut store) = build_solidity(
        r##"
        contract foo {
            function f(uint i1) public returns (int) {
                int[8] bar = [ int(10), 20, 30, 4, 5, 6, 7, 8 ];
        
                bar[2] = 0x7_f;
        
                return bar[i1];
            }

            function bar() public returns (uint) {
                uint[2][3][4] array;

                return array.length;
            }
        }"##,
    );

    runtime.constructor(&mut store, &[]);

    let val = vec![ethabi::Token::Uint(ethereum_types::U256::from(1))];

    let ret = runtime.function(&mut store, "f", &val);

    assert_eq!(ret, [ethabi::Token::Int(ethereum_types::U256::from(20))]);

    let val = vec![ethabi::Token::Uint(ethereum_types::U256::from(2))];

    let ret = runtime.function(&mut store, "f", &val);

    assert_eq!(ret, [ethabi::Token::Int(ethereum_types::U256::from(127))]);

    let ret = runtime.function(&mut store, "bar", &[]);

    assert_eq!(ret, [ethabi::Token::Uint(ethereum_types::U256::from(4))]);
}

#[test]
fn encode_array() {
    let (mut runtime, mut store) = build_solidity(
        r##"
        contract foo {
            function f(int32[4] a, uint i) public returns (int32) {
                return a[i];
            }
        }"##,
    );

    runtime.constructor(&mut store, &[]);

    let array = vec![
        ethabi::Token::Int(ethereum_types::U256::from(0x20)),
        ethabi::Token::Int(ethereum_types::U256::from(0x40)),
        ethabi::Token::Int(ethereum_types::U256::from(0x80)),
        ethabi::Token::Int(ethereum_types::U256::from(0x100)),
    ];

    for i in 0..4 {
        let ret = runtime.function(
            &mut store,
            "f",
            &[
                ethabi::Token::FixedArray(array.clone()),
                ethabi::Token::Uint(ethereum_types::U256::from(i)),
            ],
        );
        assert_eq!(ret, [array[i].clone()]);
    }
}

#[test]
#[should_panic]
fn array_bounds_uint() {
    let (mut runtime, mut store) = build_solidity(
        r##"
        contract foo {
            function f(int32[4] a, uint i) public returns (int32) {
                return a[i];
            }
        }"##,
    );

    runtime.constructor(&mut store, &[]);

    let array = vec![
        ethabi::Token::Int(ethereum_types::U256::from(0x20)),
        ethabi::Token::Int(ethereum_types::U256::from(0x40)),
        ethabi::Token::Int(ethereum_types::U256::from(0x80)),
        ethabi::Token::Int(ethereum_types::U256::from(0x100)),
    ];

    runtime.function(
        &mut store,
        "f",
        &[
            ethabi::Token::FixedArray(array),
            ethabi::Token::Uint(ethereum_types::U256::from(4)),
        ],
    );
}

fn array_bounds_int(index: ethabi::Token) {
    let (mut runtime, mut store) = build_solidity(
        r##"
        contract foo {
            function f(int32[4] a, int i) public returns (int32) {
                return a[i];
            }
        }"##,
    );

    runtime.constructor(&mut store, &[]);

    let array = vec![
        ethabi::Token::Int(ethereum_types::U256::from(0x20)),
        ethabi::Token::Int(ethereum_types::U256::from(0x40)),
        ethabi::Token::Int(ethereum_types::U256::from(0x80)),
        ethabi::Token::Int(ethereum_types::U256::from(0x100)),
    ];

    runtime.function(&mut store, "f", &[ethabi::Token::FixedArray(array), index]);
}

#[test]
#[should_panic]
fn array_bounds_int_neg() {
    array_bounds_int(ethabi::Token::Int(ethereum_types::U256::from(-1)))
}

#[test]
#[should_panic]
fn array_bounds_int_pos() {
    array_bounds_int(ethabi::Token::Int(ethereum_types::U256::from(4)))
}

#[test]
fn array_array() {
    let (mut runtime, mut store) = build_solidity(
        r##"
        contract foo {
            function f(int a, uint i1, uint i2) public returns (int) {
                    int[4][2] memory bar = [ [ int(1), 2, 3, 4 ], [ 5, 6, 7, a ] ];
    
                    return bar[i1][i2];
            }
        }"##,
    );

    runtime.constructor(&mut store, &[]);

    for i1 in 0..2 {
        for i2 in 0..4 {
            let val = runtime.function(
                &mut store,
                "f",
                &[
                    ethabi::Token::Int(ethereum_types::U256::from(8)),
                    ethabi::Token::Uint(ethereum_types::U256::from(i1)),
                    ethabi::Token::Uint(ethereum_types::U256::from(i2)),
                ],
            );

            println!("i1:{} i2:{}: {:?}", i1, i2, val);

            assert_eq!(
                val,
                [ethabi::Token::Int(ethereum_types::U256::from(
                    1 + 4 * i1 + i2
                ))]
            );
        }
    }
}

#[test]
fn arrays_are_refs() {
    // verified on remix
    let (mut runtime, mut store) = build_solidity(
        r##"
        pragma solidity >=0.4.22 <0.6.0;

        contract refs {
            function f2(int[4] memory foo) private {
                foo[2] = 2;
            }
        
            function f1(int[4] memory foo) private {
                foo[1] = 2;
            }
        
            function bar() public returns (int[4] memory) {
                int[4] memory x = [ int(0), 0, 0, 0 ];
        
                f1(x);
                f2(x);
        
                return x;
            }
        }
        "##,
    );

    runtime.constructor(&mut store, &[]);

    let val = runtime.function(&mut store, "bar", &[]);

    assert_eq!(
        val,
        &[ethabi::Token::FixedArray(vec!(
            ethabi::Token::Int(ethereum_types::U256::from(0)),
            ethabi::Token::Int(ethereum_types::U256::from(2)),
            ethabi::Token::Int(ethereum_types::U256::from(2)),
            ethabi::Token::Int(ethereum_types::U256::from(0))
        ))],
    );
}

#[test]
fn storage_structs() {
    // verified on remix
    let (mut runtime, mut store) = build_solidity(
        r##"
        pragma solidity 0;
        pragma experimental ABIEncoderV2;
        
        contract test_struct_parsing {
            struct foo {
                bool x;
                uint32 y;
            }
        
            foo f;
        
            function test() public {
                f.x = true;
                f.y = 64;
        
                assert(f.x == true);
                assert(f.y == 64);
            }
        }"##,
    );

    runtime.constructor(&mut store, &[]);

    runtime.function(&mut store, "test", &[]);
}

#[test]
fn struct_encode() {
    let (mut runtime, mut store) = build_solidity(
        r##"
        contract structs {
            struct foo {
                bool x;
                uint32 y;
            }
        
            function test(foo memory f) public {
                assert(f.x == true);
                assert(f.y == 64);
            }
        }
        "##,
    );

    runtime.constructor(&mut store, &[]);

    runtime.function(
        &mut store,
        "test",
        &[ethabi::Token::Tuple(vec![
            ethabi::Token::Bool(true),
            ethabi::Token::Uint(ethereum_types::U256::from(64)),
        ])],
    );
}

#[test]
fn struct_decode() {
    let (mut runtime, mut store) = build_solidity(
        r##"
        contract structs {
            struct foo {
                bool x;
                uint32 y;
            }
        
            function test() public returns (foo) {
                foo f;
                
                f.x = true;
                f.y = 64;

                return f;
            }
        }
        "##,
    );

    runtime.constructor(&mut store, &[]);

    let val = runtime.function(&mut store, "test", &[]);

    assert_eq!(
        val,
        &[ethabi::Token::Tuple(vec![
            ethabi::Token::Bool(true),
            ethabi::Token::Uint(ethereum_types::U256::from(64)),
        ])],
    );
}

/*
#[test]
fn struct_in_struct_decode() {
    let (runtime, mut store) = build_solidity(
        r##"
        contract structs {
            enum suit { club, diamonds, hearts, spades }
            enum value { two, three, four, five, six, seven, eight, nine, ten, jack, queen, king, ace }
            struct card {
                value v;
                suit s;
            }
            struct hand {
                card card1;
                card card2;
                card card3;
                card card4;
                card card5;
            }
            function test() public returns (hand) {
                hand h = hand({
                    card1: card({ s: suit.hearts, v: value.two }),
                    card2: card({ s: suit.diamonds, v: value.three }),
                    card3: card({ s: suit.club, v: value.four }),
                    card4: card({ s: suit.diamonds, v: value.ten }),
                    card5: card({ s: suit.hearts, v: value.jack })
                });
                return h;
            }
        }
        "##,
    );

    let val = runtime.function(&mut store, "test", &[]);

    assert_eq!(
        val,
        &[ethabi::Token::Tuple(vec![
            ethabi::Token::Tuple(vec![
                ethabi::Token::Uint(ethereum_types::U256::from(0)),
                ethabi::Token::Uint(ethereum_types::U256::from(2)),
            ]),
            ethabi::Token::Tuple(vec![
                ethabi::Token::Uint(ethereum_types::U256::from(1)),
                ethabi::Token::Uint(ethereum_types::U256::from(1)),
            ]),
            ethabi::Token::Tuple(vec![
                ethabi::Token::Uint(ethereum_types::U256::from(2)),
                ethabi::Token::Uint(ethereum_types::U256::from(0)),
            ]),
            ethabi::Token::Tuple(vec![
                ethabi::Token::Uint(ethereum_types::U256::from(8)),
                ethabi::Token::Uint(ethereum_types::U256::from(1)),
            ]),
            ethabi::Token::Tuple(vec![
                ethabi::Token::Uint(ethereum_types::U256::from(9)),
                ethabi::Token::Uint(ethereum_types::U256::from(2)),
            ]),
        ])],
    );
}*/

#[test]
fn struct_in_struct_encode() {
    let (mut runtime, mut store) = build_solidity(
        r##"
        contract structs {
            enum suit { club, diamonds, hearts, spades }
            enum value { two, three, four, five, six, seven, eight, nine, ten, jack, queen, king, ace }
            struct card {
                value v;
                suit s;
            }
            struct hand {
                card card1;
                card card2;
                card card3;
                card card4;
                card card5;
            }
        
            function test(hand h) public {
                assert(h.card1.s == suit.hearts);
                assert(h.card1.v == value.two);
                assert(h.card2.s == suit.diamonds);
                assert(h.card2.v == value.three);
                assert(h.card3.s == suit.club);
                assert(h.card3.v == value.four);
                assert(h.card4.s == suit.diamonds);
                assert(h.card4.v == value.ten);
                assert(h.card5.s == suit.hearts);
                assert(h.card5.v == value.jack);
            }
        }
        "##,
    );

    runtime.constructor(&mut store, &[]);

    runtime.function(
        &mut store,
        "test",
        &[ethabi::Token::Tuple(vec![
            ethabi::Token::Tuple(vec![
                ethabi::Token::Uint(ethereum_types::U256::from(0)),
                ethabi::Token::Uint(ethereum_types::U256::from(2)),
            ]),
            ethabi::Token::Tuple(vec![
                ethabi::Token::Uint(ethereum_types::U256::from(1)),
                ethabi::Token::Uint(ethereum_types::U256::from(1)),
            ]),
            ethabi::Token::Tuple(vec![
                ethabi::Token::Uint(ethereum_types::U256::from(2)),
                ethabi::Token::Uint(ethereum_types::U256::from(0)),
            ]),
            ethabi::Token::Tuple(vec![
                ethabi::Token::Uint(ethereum_types::U256::from(8)),
                ethabi::Token::Uint(ethereum_types::U256::from(1)),
            ]),
            ethabi::Token::Tuple(vec![
                ethabi::Token::Uint(ethereum_types::U256::from(9)),
                ethabi::Token::Uint(ethereum_types::U256::from(2)),
            ]),
        ])],
    );
}

#[test]
fn array_push_delete() {
    // ensure that structs and fixed arrays are wiped by delete
    let (mut runtime, mut store) = build_solidity(
        r#"
        pragma solidity 0;

        contract foo {
            uint32[] bar;

            function setup() public {
                for (uint32 i = 0; i < 105; i++) {
                    bar.push(i + 0x8000);
                }
            }

            function clear() public {
                delete bar;
            }
        }"#,
    );

    runtime.constructor(&mut store, &[]);

    runtime.function(&mut store, "setup", &[]);

    assert_eq!(store.store.len(), 106);

    runtime.function(&mut store, "clear", &[]);

    assert_eq!(store.store.len(), 0);
}

#[test]
fn encode_string() {
    let (mut runtime, mut store) = build_solidity(
        r##"
        contract foo {
            function f() public returns (string) {
                return "Hello, World!";
            }
        }"##,
    );

    runtime.constructor(&mut store, &[]);

    let ret = runtime.function(&mut store, "f", &[]);
    assert_eq!(ret, vec!(ethabi::Token::String("Hello, World!".to_owned())));

    let (mut runtime, mut store) = build_solidity(
        r##"
        contract foo {
            function f() public returns (int32, string, int64) {
                return (105, "the quick brown dog jumps over the lazy fox", -563);
            }
        }"##,
    );

    runtime.constructor(&mut store, &[]);

    let ret = runtime.function(&mut store, "f", &[]);

    let n563 = ethereum_types::U256::from(0)
        .overflowing_sub(ethereum_types::U256::from(563))
        .0;

    assert_eq!(
        ret,
        vec!(
            ethabi::Token::Int(ethereum_types::U256::from(105)),
            ethabi::Token::String("the quick brown dog jumps over the lazy fox".to_owned()),
            ethabi::Token::Int(n563),
        )
    );
}
