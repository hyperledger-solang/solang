extern crate ethabi;
extern crate ethereum_types;
extern crate solang;
extern crate wasmi;

use ethabi::Token;
use std::collections::HashMap;
use std::mem;
use wasmi::memory_units::Pages;
use wasmi::*;

use solang::output;
use solang::{compile, Target};

struct ContractStorage {
    memory: MemoryRef,
    store: HashMap<u32, Vec<u8>>,
}

const SET_CONTRACT_STORAGE32: usize = 0;
const GET_CONTRACT_STORAGE32: usize = 1;

impl ContractStorage {
    fn new() -> Self {
        ContractStorage {
            memory: MemoryInstance::alloc(Pages(2), Some(Pages(2))).unwrap(),
            store: HashMap::new(),
        }
    }
}

impl Externals for ContractStorage {
    fn invoke_index(
        &mut self,
        index: usize,
        args: RuntimeArgs,
    ) -> Result<Option<RuntimeValue>, Trap> {
        let slot: u32 = args.nth_checked(0)?;
        let offset: u32 = args.nth_checked(1)?;
        let len: u32 = args.nth_checked(2)?;

        match index {
            SET_CONTRACT_STORAGE32 => {
                let mut c = Vec::new();
                c.resize(len as usize, 0u8);
                if let Err(e) = self.memory.get_into(offset, &mut c) {
                    panic!("set_storage32: {}", e);
                }
                self.store.insert(slot, c);
            }
            GET_CONTRACT_STORAGE32 => {
                let mut c = Vec::new();
                if let Some(k) = self.store.get(&slot) {
                    c = k.clone();
                }
                c.resize(len as usize, 0u8);

                if let Err(e) = self.memory.set(offset, &c) {
                    panic!("get_storage32: {}", e);
                }
            }
            _ => panic!("external {} unknown", index),
        }

        Ok(None)
    }
}

impl ModuleImportResolver for ContractStorage {
    fn resolve_func(&self, field_name: &str, signature: &Signature) -> Result<FuncRef, Error> {
        let index = match field_name {
            "set_storage32" => SET_CONTRACT_STORAGE32,
            "get_storage32" => GET_CONTRACT_STORAGE32,
            _ => {
                panic!("{} not implemented", field_name);
            }
        };

        Ok(FuncInstance::alloc_host(signature.clone(), index))
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
        let calldata = self.abi.functions[name].encode_input(args).unwrap();
        // need to prepend length
        store.memory.set_value(0, calldata.len() as u32).unwrap();
        store
            .memory
            .set(mem::size_of::<u32>() as u32, &calldata)
            .unwrap();

        let ret = self
            .module
            .invoke_export("function", &[RuntimeValue::I32(0)], store)
            .expect("failed to call function");

        match ret {
            Some(RuntimeValue::I32(offset)) => {
                let offset = offset as u32;
                let returndata = store
                    .memory
                    .get(offset + mem::size_of::<u32>() as u32, 32)
                    .unwrap();

                println!("RETURNDATA: {}", hex::encode(&returndata));

                self.abi.functions[name].decode_output(&returndata).unwrap()
            }
            _ => panic!("expected return value when calling {}", name),
        }
    }

    fn constructor(&self, store: &mut ContractStorage, args: &[Token]) {
        if let Some(constructor) = &self.abi.constructor {
            let calldata = constructor.encode_input(Vec::new(), args).unwrap();

            // need to prepend length
            store.memory.set_value(0, calldata.len() as u32).unwrap();
            store
                .memory
                .set(mem::size_of::<u32>() as u32, &calldata)
                .unwrap();

            let ret = self
                .module
                .invoke_export("constructor", &[RuntimeValue::I32(0)], store)
                .expect("failed to call constructor");

            match ret {
                None => (),
                _ => panic!("not expected return value when calling constructor"),
            }
        }
    }
}

fn build_solidity(src: &'static str) -> (TestRuntime, ContractStorage) {
    let (mut res, errors) = compile(src, "test.sol", "default", &Target::Burrow);

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
                &ImportsBuilder::new().with_resolver("env", &store),
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
    let (runtime, mut store) = build_solidity(
        "
        contract test {
            function foo() public returns (uint32) {
                return 2;
            }
        }",
    );

    let returns = runtime.function(&mut store, "foo", &[]);

    assert_eq!(
        returns,
        vec![ethabi::Token::Uint(ethereum_types::U256::from(2))]
    );
}

#[test]
fn simple_loops() {
    let (runtime, mut store) = build_solidity(
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
    let (runtime, mut store) = build_solidity(
        r##"
contract test3 {
	function foo() public returns (bool) {
		uint b = 18446744073709551616;
    uint c = 36893488147419103232;

    return b * 2 == c;
	}
}"##,
    );

    let returns = runtime.function(&mut store, "foo", &[]);

    assert_eq!(returns, vec![ethabi::Token::Bool(true)]);
}

#[test]
fn abi_call_return_test() {
    let (runtime, mut store) = build_solidity(
        r##"
contract test {
	function foo() public returns (uint32) {
    return 102;
	}
}"##,
    );

    let returns = runtime.function(&mut store, "foo", &[]);

    assert_eq!(
        returns,
        vec![ethabi::Token::Uint(ethereum_types::U256::from(102))]
    );
}

#[test]
fn abi_call_pass_return_test() {
    let (runtime, mut store) = build_solidity(
        r##"
contract test {
	function foo(uint32 a) public returns (uint32) {
    return a;
	}
}"##,
    );

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
    let (runtime, mut store) = build_solidity(
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
    let (runtime, mut store) = build_solidity(
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
    let (runtime, mut store) = build_solidity(
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
    let (runtime, mut store) = build_solidity(
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
