extern crate ethabi;
extern crate ethereum_types;
extern crate num_derive;
extern crate num_traits;
extern crate rand;
extern crate solang;
extern crate wasmi;

use ethabi::{decode, Token};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use rand::Rng;
use std::collections::HashMap;
use std::fmt;
use wasmi::memory_units::Pages;
use wasmi::*;

use solang::output;
use solang::{compile, Target};

type Address = [u8; 20];

fn address_new() -> Address {
    let mut rng = rand::thread_rng();

    let mut a = [0u8; 20];

    rng.fill(&mut a[..]);

    a
}

struct VM {
    memory: MemoryRef,
    cur: Address,
    code: Vec<u8>,
    input: Vec<u8>,
    output: Vec<u8>,
    returndata: Vec<u8>,
}

impl VM {
    fn new(code: Vec<u8>, address: Address) -> Self {
        VM {
            memory: MemoryInstance::alloc(Pages(2), Some(Pages(2))).unwrap(),
            input: Vec::new(),
            output: Vec::new(),
            returndata: Vec::new(),
            code,
            cur: address,
        }
    }
}

struct TestRuntime {
    abi: ethabi::Contract,
    contracts: Vec<Vec<u8>>,
    value: u128,
    accounts: HashMap<Address, (Vec<u8>, u128)>,
    store: HashMap<(Address, [u8; 32]), [u8; 32]>,
    vm: VM,
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
    getCodeSize,
    codeCopy,
    create,
    call,
    returnDataCopy,
    getReturnDataSize,
    getCallValue,
    getAddress,
    getExternalBalance,
    selfDestruct,
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

impl Externals for TestRuntime {
    fn invoke_index(
        &mut self,
        index: usize,
        args: RuntimeArgs,
    ) -> Result<Option<RuntimeValue>, Trap> {
        match FromPrimitive::from_usize(index) {
            Some(Extern::getCallDataSize) => {
                Ok(Some(RuntimeValue::I32(self.vm.input.len() as i32)))
            }
            Some(Extern::getCodeSize) => Ok(Some(RuntimeValue::I32(self.vm.code.len() as i32))),
            Some(Extern::getReturnDataSize) => {
                Ok(Some(RuntimeValue::I32(self.vm.returndata.len() as i32)))
            }
            Some(Extern::callDataCopy) => {
                let dest = args.nth_checked::<u32>(0)?;
                let input_offset = args.nth_checked::<u32>(1)? as usize;
                let input_len = args.nth_checked::<u32>(2)? as usize;

                self.vm
                    .memory
                    .set(
                        dest,
                        &self.vm.input[input_offset as usize..input_offset + input_len],
                    )
                    .expect("calldatacopy should work");

                Ok(None)
            }
            Some(Extern::codeCopy) => {
                let dest = args.nth_checked::<u32>(0)?;
                let code_offset = args.nth_checked::<u32>(1)? as usize;
                let code_len = args.nth_checked::<u32>(2)? as usize;

                let data = &self.vm.code[code_offset as usize..code_offset + code_len];

                println!("codeCopy {} {}", code_len, hex::encode(data));

                self.vm
                    .memory
                    .set(dest, data)
                    .expect("codeCopy should work");

                Ok(None)
            }
            Some(Extern::returnDataCopy) => {
                let dest = args.nth_checked::<u32>(0)?;
                let data_offset = args.nth_checked::<u32>(1)? as usize;
                let data_len = args.nth_checked::<u32>(2)? as usize;

                let data = &self.vm.returndata[data_offset as usize..data_offset + data_len];

                println!("returnDataCopy {} {}", data_len, hex::encode(data));

                self.vm
                    .memory
                    .set(dest, data)
                    .expect("returnDataCopy should work");

                Ok(None)
            }
            Some(Extern::finish) => {
                let src: u32 = args.nth_checked(0)?;
                let len: u32 = args.nth_checked(1)?;

                let mut output = Vec::new();
                output.resize(len as usize, 0);

                self.vm.memory.get_into(src, &mut output).unwrap();

                println!("finish: {} {}", len, hex::encode(&output));

                self.vm.output = output;

                Err(Trap::new(TrapKind::Host(Box::new(HostCodeFinish {}))))
            }
            Some(Extern::revert) => {
                let src: u32 = args.nth_checked(0)?;
                let len: u32 = args.nth_checked(1)?;

                let mut output = Vec::new();
                output.resize(len as usize, 0);

                self.vm.memory.get_into(src, &mut output).unwrap();
                self.vm.output = output;

                println!(
                    "revert {} {}",
                    self.vm.output.len(),
                    hex::encode(&self.vm.output)
                );

                Err(Trap::new(TrapKind::Host(Box::new(HostCodeRevert {}))))
            }
            Some(Extern::storageLoad) => {
                let key_ptr: u32 = args.nth_checked(0)?;
                let data_ptr: u32 = args.nth_checked(1)?;

                let mut key = [0u8; 32];

                self.vm
                    .memory
                    .get_into(key_ptr, &mut key)
                    .expect("copy key from wasm memory");

                let res = if let Some(v) = self.store.get(&(self.vm.cur, key)) {
                    v
                } else {
                    &[0u8; 32]
                };
                self.vm
                    .memory
                    .set(data_ptr, res)
                    .expect("copy key from wasm memory");

                Ok(None)
            }
            Some(Extern::storageStore) => {
                let key_ptr: u32 = args.nth_checked(0)?;
                let data_ptr: u32 = args.nth_checked(1)?;

                let mut key = [0u8; 32];
                let mut data = [0u8; 32];

                self.vm
                    .memory
                    .get_into(key_ptr, &mut key)
                    .expect("copy key from wasm memory");

                self.vm
                    .memory
                    .get_into(data_ptr, &mut data)
                    .expect("copy key from wasm memory");

                if data.iter().any(|n| *n != 0) {
                    self.store.insert((self.vm.cur, key), data);
                } else {
                    self.store.remove(&(self.vm.cur, key));
                }
                Ok(None)
            }
            Some(Extern::printMem) => {
                let data_ptr: u32 = args.nth_checked(0)?;
                let len: u32 = args.nth_checked(1)?;

                let mut buf = Vec::new();
                buf.resize(len as usize, 0u8);

                if let Err(e) = self.vm.memory.get_into(data_ptr, &mut buf) {
                    panic!("printMem: {}", e);
                }

                println!("{}", String::from_utf8_lossy(&buf));

                Ok(None)
            }
            Some(Extern::create) => {
                //let balance_ptr: u32 = args.nth_checked(0)?;
                let input_ptr: u32 = args.nth_checked(1)?;
                let input_len: u32 = args.nth_checked(2)?;
                let address_ptr: u32 = args.nth_checked(3)?;

                let mut buf = Vec::new();
                buf.resize(input_len as usize, 0u8);

                if let Err(e) = self.vm.memory.get_into(input_ptr, &mut buf) {
                    panic!("create: {}", e);
                }

                println!("create code: {}", hex::encode(&buf));

                let addr = address_new();
                println!("create address: {}", hex::encode(&addr));

                // when ewasm creates a contract, the abi encoded args are concatenated to the
                // code. So, find which code is was and use that instead. Otherwise, the
                // wasm validator will trip
                let code = self
                    .contracts
                    .iter()
                    .find(|c| buf.starts_with(c))
                    .unwrap()
                    .clone();

                let mut vm = VM::new(buf, addr);

                std::mem::swap(&mut self.vm, &mut vm);

                let module = self.create_module(&code);

                match module.invoke_export("main", &[], self) {
                    Err(wasmi::Error::Trap(trap)) => match trap.kind() {
                        TrapKind::Host(host_error) => {
                            if host_error.downcast_ref::<HostCodeRevert>().is_some() {
                                panic!("revert executed");
                            }
                        }
                        _ => panic!("fail to invoke main via create: {}", trap),
                    },
                    Ok(_) => {}
                    Err(e) => panic!("fail to invoke main via create: {}", e),
                }

                let res = self.vm.output.clone();

                println!("create returns: {}", hex::encode(&res));

                std::mem::swap(&mut self.vm, &mut vm);

                self.accounts.insert(addr, (res, 0));

                self.vm
                    .memory
                    .set(address_ptr, &addr[..])
                    .expect("copy key from wasm memory");

                Ok(Some(RuntimeValue::I32(0)))
            }
            Some(Extern::call) => {
                //let gas: u64 = args.nth_checked(0)?;
                let address_ptr: u32 = args.nth_checked(1)?;
                //let value_ptr: u32 = args.nth_checked(2)?;
                let input_ptr: u32 = args.nth_checked(3)?;
                let input_len: u32 = args.nth_checked(4)?;

                let mut buf = Vec::new();
                buf.resize(input_len as usize, 0u8);

                if let Err(e) = self.vm.memory.get_into(input_ptr, &mut buf) {
                    panic!("call: {}", e);
                }

                let mut addr = [0u8; 20];

                if let Err(e) = self.vm.memory.get_into(address_ptr, &mut addr) {
                    panic!("call: {}", e);
                }

                println!(
                    "extern call address: {} data: {}",
                    hex::encode(&addr),
                    hex::encode(&buf)
                );

                // when ewasm creates a contract, the abi encoded args are concatenated to the
                // code. So, find which code is was and use that instead. Otherwise, the
                // wasm validator will trip
                let (code, _) = self.accounts.get(&addr).unwrap().clone();

                let mut vm = VM::new(code.to_vec(), addr);

                std::mem::swap(&mut self.vm, &mut vm);

                self.vm.input = buf;

                let module = self.create_module(&code);

                let ret = match module.invoke_export("main", &[], self) {
                    Err(wasmi::Error::Trap(trap)) => match trap.kind() {
                        TrapKind::Host(kind) => {
                            if format!("{}", kind) == "revert" {
                                1
                            } else {
                                0
                            }
                        }
                        _ => panic!("fail to invoke main via create: {}", trap),
                    },
                    Ok(_) => 0,
                    Err(e) => panic!("fail to invoke main via create: {}", e),
                };

                let res = self.vm.output.clone();

                std::mem::swap(&mut self.vm, &mut vm);

                self.vm.returndata = res;

                self.vm
                    .memory
                    .set(address_ptr, &addr[..])
                    .expect("copy key from wasm memory");

                Ok(Some(RuntimeValue::I32(ret)))
            }
            Some(Extern::getCallValue) => {
                let value_ptr: u32 = args.nth_checked(0)?;

                let value = self.value.to_le_bytes();

                println!("getCallValue: {}", hex::encode(&value));

                self.vm.memory.set(value_ptr, &value).expect("set value");

                Ok(None)
            }
            Some(Extern::getAddress) => {
                let address_ptr: u32 = args.nth_checked(0)?;

                println!("getAddress: {}", hex::encode(&self.vm.cur));

                self.vm
                    .memory
                    .set(address_ptr, &self.vm.cur[..])
                    .expect("set address");

                Ok(None)
            }
            Some(Extern::getExternalBalance) => {
                let address_ptr: u32 = args.nth_checked(0)?;
                let balance_ptr: u32 = args.nth_checked(1)?;

                let mut addr = [0u8; 20];

                if let Err(e) = self.vm.memory.get_into(address_ptr, &mut addr) {
                    panic!("call: {}", e);
                }

                let value = self.accounts.get(&addr).map(|a| a.1).unwrap_or(0);

                println!("getExternalBalance: {} {}", hex::encode(&addr), value);

                self.vm
                    .memory
                    .set(balance_ptr, &value.to_le_bytes()[..])
                    .expect("set balance");

                Ok(None)
            }
            Some(Extern::selfDestruct) => {
                let address_ptr: u32 = args.nth_checked(0)?;

                let mut addr = [0u8; 20];

                if let Err(e) = self.vm.memory.get_into(address_ptr, &mut addr) {
                    panic!("selfDestruct: {}", e);
                }

                let remaining = self.accounts[&self.vm.cur].1;

                self.accounts.get_mut(&addr).unwrap().1 += remaining;

                println!("selfDestruct: {} {}", hex::encode(&addr), remaining);

                self.accounts.remove(&self.vm.cur);

                Err(Trap::new(TrapKind::Host(Box::new(HostCodeFinish {}))))
            }
            _ => panic!("external {} unknown", index),
        }
    }
}

impl ModuleImportResolver for TestRuntime {
    fn resolve_func(&self, field_name: &str, signature: &Signature) -> Result<FuncRef, Error> {
        let index = match field_name {
            "getCallDataSize" => Extern::getCallDataSize,
            "callDataCopy" => Extern::callDataCopy,
            "finish" => Extern::finish,
            "revert" => Extern::revert,
            "storageStore" => Extern::storageStore,
            "storageLoad" => Extern::storageLoad,
            "printMem" => Extern::printMem,
            "getCodeSize" => Extern::getCodeSize,
            "codeCopy" => Extern::codeCopy,
            "create" => Extern::create,
            "call" => Extern::call,
            "returnDataCopy" => Extern::returnDataCopy,
            "getReturnDataSize" => Extern::getReturnDataSize,
            "getCallValue" => Extern::getCallValue,
            "getAddress" => Extern::getAddress,
            "getExternalBalance" => Extern::getExternalBalance,
            "selfDestruct" => Extern::selfDestruct,
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

impl TestRuntime {
    fn create_module(&self, code: &[u8]) -> ModuleRef {
        let module = Module::from_buffer(&code).expect("parse wasm should work");

        ModuleInstance::new(
            &module,
            &ImportsBuilder::new().with_resolver("ethereum", self),
        )
        .expect("Failed to instantiate module")
        .run_start(&mut NopExternals)
        .expect("Failed to run start function in module")
    }

    fn function(&mut self, name: &str, args: &[Token]) -> Vec<Token> {
        let calldata = match self.abi.functions[name][0].encode_input(args) {
            Ok(n) => n,
            Err(x) => panic!(format!("{}", x)),
        };

        let module = self.create_module(&self.accounts[&self.vm.cur].0);

        println!("FUNCTION CALLDATA: {}", hex::encode(&calldata));

        self.vm.input = calldata;

        match module.invoke_export("main", &[], self) {
            Err(wasmi::Error::Trap(trap)) => match trap.kind() {
                TrapKind::Host(_) => {}
                _ => panic!("fail to invoke main: {}", trap),
            },
            Ok(Some(RuntimeValue::I32(0))) => {}
            Ok(Some(RuntimeValue::I32(ret))) => panic!("main returns: {}", ret),
            Err(e) => panic!("fail to invoke main: {}", e),
            Ok(None) => panic!("fail to invoke main"),
            _ => panic!("fail to invoke main, unknown"),
        }

        println!("RETURNDATA: {}", hex::encode(&self.vm.output));

        self.abi.functions[name][0]
            .decode_output(&self.vm.output)
            .unwrap()
    }

    fn function_abi_fail(&mut self, name: &str, args: &[Token], patch: fn(&mut Vec<u8>)) {
        let mut calldata = match self.abi.functions[name][0].encode_input(args) {
            Ok(n) => n,
            Err(x) => panic!(format!("{}", x)),
        };

        patch(&mut calldata);

        let module = self.create_module(&self.accounts[&self.vm.cur].0);

        println!("FUNCTION CALLDATA: {}", hex::encode(&calldata));

        self.vm.input = calldata;

        match module.invoke_export("main", &[], self) {
            Err(wasmi::Error::Trap(trap)) => match trap.kind() {
                TrapKind::Host(_) => {}
                _ => panic!("fail to invoke main: {}", trap),
            },
            Ok(Some(RuntimeValue::I32(3))) => {}
            Ok(Some(RuntimeValue::I32(ret))) => panic!("main returns: {}", ret),
            Err(e) => panic!("fail to invoke main: {}", e),
            Ok(None) => panic!("fail to invoke main"),
            _ => panic!("fail to invoke main, unknown"),
        }
    }

    fn function_revert(&mut self, name: &str, args: &[Token]) -> Option<String> {
        let calldata = match self.abi.functions[name][0].encode_input(args) {
            Ok(n) => n,
            Err(x) => panic!(format!("{}", x)),
        };

        let module = self.create_module(&self.accounts[&self.vm.cur].0);

        println!("FUNCTION CALLDATA: {}", hex::encode(&calldata));

        self.vm.input = calldata;

        match module.invoke_export("main", &[], self) {
            Err(wasmi::Error::Trap(trap)) => match trap.kind() {
                TrapKind::Host(host_error) => {
                    if host_error.downcast_ref::<HostCodeFinish>().is_some() {
                        panic!("function was suppose to revert, not finish")
                    }
                }
                _ => panic!("fail to invoke main: {}", trap),
            },
            Ok(Some(RuntimeValue::I32(1))) => {}
            Err(e) => panic!("fail to invoke main: {}", e),
            _ => panic!("fail to invoke main"),
        }

        println!("RETURNDATA: {}", hex::encode(&self.vm.output));

        if self.vm.output.is_empty() {
            return None;
        }

        assert_eq!(self.vm.output[..4], 0x08c3_79a0u32.to_be_bytes());

        if let Ok(v) = decode(&[ethabi::ParamType::String], &self.vm.output[4..]) {
            assert_eq!(v.len(), 1);

            if let ethabi::Token::String(r) = &v[0] {
                return Some(r.to_owned());
            }
        }

        panic!("failed to decode");
    }

    fn constructor_expect_revert(&mut self, args: &[Token]) {
        assert!(!self.do_constructor(args));
    }

    fn constructor(&mut self, args: &[Token]) {
        assert!(self.do_constructor(args));
    }

    fn do_constructor(&mut self, args: &[Token]) -> bool {
        let calldata = if let Some(constructor) = &self.abi.constructor {
            constructor.encode_input(Vec::new(), args).unwrap()
        } else {
            Vec::new()
        };

        let module = self.create_module(self.contracts.last().unwrap());

        println!("CONSTRUCTOR CALLDATA: {}", hex::encode(&calldata));

        self.vm.code.extend(calldata);
        self.vm.cur = address_new();

        match module.invoke_export("main", &[], self) {
            Err(wasmi::Error::Trap(trap)) => match trap.kind() {
                TrapKind::Host(host_error) => {
                    if host_error.downcast_ref::<HostCodeRevert>().is_some() {
                        return false;
                    }
                }
                _ => panic!("fail to invoke main: {}", trap),
            },
            Ok(_) => {}
            Err(e) => panic!("fail to invoke main: {}", e),
        }

        println!(
            "DEPLOYER RETURNS: {} {}",
            self.vm.output.len(),
            hex::encode(&self.vm.output)
        );

        self.accounts
            .insert(self.vm.cur, (self.vm.output.clone(), 0));

        true
    }
}

fn build_solidity(src: &'static str) -> TestRuntime {
    let (res, errors) = compile(
        src,
        "test.sol",
        inkwell::OptimizationLevel::Default,
        Target::Ewasm,
    );

    output::print_messages("test.sol", src, &errors, false);

    for v in &res {
        println!("contract size:{}", v.0.len());
    }

    assert_eq!(res.is_empty(), false);

    // resolve
    let (bc, abi) = res.last().unwrap().clone();

    TestRuntime {
        accounts: HashMap::new(),
        vm: VM::new(bc, [0u8; 20]),
        value: 0,
        store: HashMap::new(),
        abi: ethabi::Contract::load(abi.as_bytes()).unwrap(),
        contracts: res.into_iter().map(|v| v.0).collect(),
    }
}

#[test]
fn simple_solidiy_compile_and_run() {
    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            function foo() public returns (uint32) {
                return 2;
            }
        }",
    );

    // call constructor
    runtime.constructor(&[]);

    let returns = runtime.function("foo", &[]);

    assert_eq!(
        returns,
        vec![ethabi::Token::Uint(ethereum_types::U256::from(2))]
    );
}

#[test]
fn simple_loops() {
    let mut runtime = build_solidity(
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
    runtime.constructor(&[]);

    for i in 0..=50 {
        let res = ((50 - i) * 100 + 5) + i * 1000;

        let returns =
            runtime.function("foo", &[ethabi::Token::Uint(ethereum_types::U256::from(i))]);

        assert_eq!(
            returns,
            vec![ethabi::Token::Uint(ethereum_types::U256::from(res))]
        );
    }

    for i in 0..=50 {
        let res = (i + 1) * 10 + 1;

        let returns = runtime.function(
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

        let returns =
            runtime.function("baz", &[ethabi::Token::Uint(ethereum_types::U256::from(i))]);

        assert_eq!(
            returns,
            vec![ethabi::Token::Uint(ethereum_types::U256::from(res))]
        );
    }
}

#[test]
fn stack_test() {
    let mut runtime = build_solidity(
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
    runtime.constructor(&[]);

    let returns = runtime.function("foo", &[]);

    assert_eq!(returns, vec![ethabi::Token::Bool(true)]);
}

#[test]
fn abi_call_return_test() {
    let mut runtime = build_solidity(
        r##"
contract test {
	function foo() public returns (uint32) {
    return 102;
	}
}"##,
    );

    // call constructor
    runtime.constructor(&[]);

    let returns = runtime.function("foo", &[]);

    assert_eq!(
        returns,
        vec![ethabi::Token::Uint(ethereum_types::U256::from(102))]
    );
}

#[test]
fn abi_call_pass_return_test() {
    let mut runtime = build_solidity(
        r##"
        contract x {
            function test() public {

            }
        }

        contract bar {
            function foo(uint32 a) public returns (uint32) {
            return a;
            }
        }"##,
    );

    // call constructor
    runtime.constructor(&[]);

    for val in [102i32, 255, 256, 0x7fff_ffff].iter() {
        let returns = runtime.function(
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
    let mut runtime = build_solidity(
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
    runtime.constructor(&[]);

    for val in [4096u32, 1000u32].iter() {
        let eval = ethabi::Token::Uint(ethereum_types::U256::from(*val));
        // create call for foo
        let returns = runtime.function("setFoo", &[eval]);

        assert_eq!(returns, vec![]);

        // create call for foo
        let returns = runtime.function("getFoo", &[]);

        let eval = ethabi::Token::Uint(ethereum_types::U256::from(*val));
        assert_eq!(returns, vec![eval]);
    }
}

#[test]
fn large_ints_encoded() {
    let mut runtime = build_solidity(
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
    runtime.constructor(&[]);

    for val in [4096u32, 1000u32].iter() {
        let eval = ethabi::Token::Uint(ethereum_types::U256::from(*val));
        // create call for foo
        let returns = runtime.function("setFoo", &[eval]);

        assert_eq!(returns, vec![]);

        // create call for foo
        let returns = runtime.function("getFoo", &[]);

        let eval = ethabi::Token::Uint(ethereum_types::U256::from(*val));
        assert_eq!(returns, vec![eval]);
    }
}

#[test]
fn address() {
    let mut runtime = build_solidity(
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
    runtime.constructor(&[]);

    let ret = runtime.function("encode_const", &[]);

    assert_eq!(
        ret,
        [ethabi::Token::Address(ethereum_types::Address::from_slice(
            &hex::decode("52908400098527886E0F7030069857D2E4169EE7").unwrap()
        ))]
    );

    runtime.function(
        "test_arg",
        &[ethabi::Token::Address(ethereum_types::Address::from_slice(
            &hex::decode("27b1fdb04752bbc536007a920d24acb045561c26").unwrap(),
        ))],
    );

    let ret = runtime.function("allones", &[]);

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
    let mut runtime = build_solidity(
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

    runtime.constructor(&[]);

    let ret = runtime.function("get_foo", &[]);

    assert_eq!(
        ret,
        [ethabi::Token::FixedBytes(vec!(0x11, 0x22, 0x33, 0x44))]
    );

    let ret = runtime.function("bytes4asuint32", &[]);

    assert_eq!(
        ret,
        [ethabi::Token::Uint(ethereum_types::U256::from(
            0x11_22_33_44
        ))]
    );

    let ret = runtime.function("bytes4asuint64", &[]);

    assert_eq!(
        ret,
        [ethabi::Token::Uint(ethereum_types::U256::from(
            0x1122_3344_0000_0000u64
        ))]
    );

    let ret = runtime.function("bytes4asbytes2", &[]);

    assert_eq!(ret, [ethabi::Token::FixedBytes(vec!(0x11, 0x22))]);

    let val = vec![ethabi::Token::FixedBytes(vec![0x41, 0x42, 0x43, 0x44])];

    assert_eq!(runtime.function("passthrough", &val), val);

    let val = vec![ethabi::Token::Uint(ethereum_types::U256::from(1))];

    let ret = runtime.function("entry", &val);

    assert_eq!(ret, [ethabi::Token::FixedBytes(vec!(0x22))]);

    let ret = runtime.function("entry2", &val);

    assert_eq!(ret, [ethabi::Token::FixedBytes(vec!(0xBB))]);
}

#[test]
fn array() {
    let mut runtime = build_solidity(
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

    runtime.constructor(&[]);

    let val = vec![ethabi::Token::Uint(ethereum_types::U256::from(1))];

    let ret = runtime.function("f", &val);

    assert_eq!(ret, [ethabi::Token::Int(ethereum_types::U256::from(20))]);

    let val = vec![ethabi::Token::Uint(ethereum_types::U256::from(2))];

    let ret = runtime.function("f", &val);

    assert_eq!(ret, [ethabi::Token::Int(ethereum_types::U256::from(127))]);

    let ret = runtime.function("bar", &[]);

    assert_eq!(ret, [ethabi::Token::Uint(ethereum_types::U256::from(4))]);
}

#[test]
fn encode_array() {
    let mut runtime = build_solidity(
        r##"
        contract foo {
            function f(int32[4] a, uint i) public returns (int32) {
                return a[i];
            }
        }"##,
    );

    runtime.constructor(&[]);

    let array = vec![
        ethabi::Token::Int(ethereum_types::U256::from(0x20)),
        ethabi::Token::Int(ethereum_types::U256::from(0x40)),
        ethabi::Token::Int(ethereum_types::U256::from(0x80)),
        ethabi::Token::Int(ethereum_types::U256::from(0x100)),
    ];

    for i in 0..4 {
        let ret = runtime.function(
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
    let mut runtime = build_solidity(
        r##"
        contract foo {
            function f(int32[4] a, uint i) public returns (int32) {
                return a[i];
            }
        }"##,
    );

    runtime.constructor(&[]);

    let array = vec![
        ethabi::Token::Int(ethereum_types::U256::from(0x20)),
        ethabi::Token::Int(ethereum_types::U256::from(0x40)),
        ethabi::Token::Int(ethereum_types::U256::from(0x80)),
        ethabi::Token::Int(ethereum_types::U256::from(0x100)),
    ];

    runtime.function(
        "f",
        &[
            ethabi::Token::FixedArray(array),
            ethabi::Token::Uint(ethereum_types::U256::from(4)),
        ],
    );
}

fn array_bounds_int(index: ethabi::Token) {
    let mut runtime = build_solidity(
        r##"
        contract foo {
            function f(int32[4] a, int i) public returns (int32) {
                return a[i];
            }
        }"##,
    );

    runtime.constructor(&[]);

    let array = vec![
        ethabi::Token::Int(ethereum_types::U256::from(0x20)),
        ethabi::Token::Int(ethereum_types::U256::from(0x40)),
        ethabi::Token::Int(ethereum_types::U256::from(0x80)),
        ethabi::Token::Int(ethereum_types::U256::from(0x100)),
    ];

    runtime.function("f", &[ethabi::Token::FixedArray(array), index]);
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
    let mut runtime = build_solidity(
        r##"
        contract foo {
            function f(int a, uint i1, uint i2) public returns (int) {
                    int[4][2] memory bar = [ [ int(1), 2, 3, 4 ], [ 5, 6, 7, a ] ];
    
                    return bar[i1][i2];
            }
        }"##,
    );

    runtime.constructor(&[]);

    for i1 in 0..2 {
        for i2 in 0..4 {
            let val = runtime.function(
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
    let mut runtime = build_solidity(
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

    runtime.constructor(&[]);

    let val = runtime.function("bar", &[]);

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
    let mut runtime = build_solidity(
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

    runtime.constructor(&[]);

    runtime.function("test", &[]);
}

#[test]
fn struct_encode() {
    let mut runtime = build_solidity(
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

    runtime.constructor(&[]);

    runtime.function(
        "test",
        &[ethabi::Token::Tuple(vec![
            ethabi::Token::Bool(true),
            ethabi::Token::Uint(ethereum_types::U256::from(64)),
        ])],
    );
}

#[test]
fn struct_dynamic_array_encode() {
    let mut runtime = build_solidity(
        r##"
        contract structs {
            struct foo {
                bool x;
                uint32 y;
            }
        
            function test() public returns (foo[]) {
                foo[] x = new foo[](3);

                x[0] = foo({x: true,y: 64});
                x[1] = foo({x: false,y: 102});
                x[2] = foo({x: true,y: 0x800});

                return x;
            }
        }
        "##,
    );

    runtime.constructor(&[]);

    let ret = runtime.function("test", &[]);

    assert_eq!(
        ret,
        vec![ethabi::Token::Array(vec![
            ethabi::Token::Tuple(vec![
                ethabi::Token::Bool(true),
                ethabi::Token::Uint(ethereum_types::U256::from(64))
            ]),
            ethabi::Token::Tuple(vec![
                ethabi::Token::Bool(false),
                ethabi::Token::Uint(ethereum_types::U256::from(102))
            ]),
            ethabi::Token::Tuple(vec![
                ethabi::Token::Bool(true),
                ethabi::Token::Uint(ethereum_types::U256::from(0x800)),
            ])
        ])],
    );
}

#[test]
fn struct_decode() {
    let mut runtime = build_solidity(
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

    runtime.constructor(&[]);

    let val = runtime.function("test", &[]);

    assert_eq!(
        val,
        &[ethabi::Token::Tuple(vec![
            ethabi::Token::Bool(true),
            ethabi::Token::Uint(ethereum_types::U256::from(64)),
        ])],
    );
}

/* TODO: find out why this test fails.
#[test]
fn struct_in_struct_decode() {
    let mut runtime = build_solidity(
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

    runtime.constructor(&[]);

    let val = runtime.function("test", &[]);

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
    let mut runtime = build_solidity(
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

    runtime.constructor(&[]);

    runtime.function(
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
    let mut runtime = build_solidity(
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

    runtime.constructor(&[]);

    runtime.function("setup", &[]);

    assert_eq!(runtime.store.len(), 106);

    runtime.function("clear", &[]);

    assert_eq!(runtime.store.len(), 0);
}

#[test]
fn encode_string() {
    let mut runtime = build_solidity(
        r##"
        contract foo {
            function f() public returns (string) {
                return "Hello, World!";
            }
        }"##,
    );

    runtime.constructor(&[]);

    let ret = runtime.function("f", &[]);
    assert_eq!(ret, vec!(ethabi::Token::String("Hello, World!".to_owned())));

    let mut runtime = build_solidity(
        r##"
        contract foo {
            function f() public returns (int32, string, int64) {
                return (105, "the quick brown dog jumps over the lazy fox", -563);
            }
        }"##,
    );

    runtime.constructor(&[]);

    let ret = runtime.function("f", &[]);

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

#[test]
fn decode_string() {
    let mut runtime = build_solidity(
        r##"
        contract foo {
            function f(string a) public returns (string) {
                return a + " ";
            }
        }"##,
    );

    runtime.constructor(&[]);

    let ret = runtime.function("f", &[ethabi::Token::String("Hello, World!".to_owned())]);

    assert_eq!(
        ret,
        vec!(ethabi::Token::String("Hello, World! ".to_owned()))
    );
}

#[test]
fn revert() {
    let mut runtime = build_solidity(
        r##"
        contract foo {
            function f() public {
                revert("Hello, World!");
            }
        }"##,
    );

    runtime.constructor(&[]);

    let ret = runtime.function_revert("f", &[]);
    assert_eq!(ret, Some("Hello, World!".to_owned()));
}

#[test]
fn constructor_args() {
    let mut runtime = build_solidity(
        r##"
        contract foo {
            int64 v;

            constructor(int64 a) public {
                v = a;
            }

            function f() public returns (int64) {
                return v;
            }
        }"##,
    );

    let v = ethabi::Token::Int(ethereum_types::U256::from(105));

    runtime.constructor(&[v.clone()]);

    let ret = runtime.function("f", &[]);
    assert_eq!(ret, vec!(v));
}

#[test]
fn create() {
    let mut runtime = build_solidity(
        r##"
        contract a {
            int32 x;
            constructor() public {
            }

            function test() public {
                x = 102;
            }
        }

        contract b {
            function x() public {
                a r = new a();
            }
        }
        "##,
    );

    runtime.constructor(&[]);

    runtime.function("x", &[]);
}

#[test]
fn decode_complexish() {
    let mut runtime = build_solidity(
        r##"
        pragma solidity 0;

        struct foo1 {
            int32 f1;
            string f2;
            int64[2] f3;
            int64[] f4;
        }

        contract c {
            function test(foo1[] a) public {
                assert(a.length == 1);
                assert(a[0].f2 == "Hello, World!");
                assert(a[0].f3[0] == 55);
                assert(a[0].f3[1] == 59);
                assert(a[0].f4.length == 1);
                assert(a[0].f4[0] == 102);
            }
        }"##,
    );

    runtime.constructor(&[]);

    runtime.function(
        "test",
        &[ethabi::Token::Array(vec![ethabi::Token::Tuple(vec![
            ethabi::Token::Int(ethereum_types::U256::from(102)),
            ethabi::Token::String("Hello, World!".to_owned()),
            ethabi::Token::FixedArray(vec![
                ethabi::Token::Int(ethereum_types::U256::from(55)),
                ethabi::Token::Int(ethereum_types::U256::from(59)),
            ]),
            ethabi::Token::Array(vec![ethabi::Token::Int(ethereum_types::U256::from(102))]),
        ])])],
    );
}

#[test]
fn decode_bad_abi() {
    let mut runtime = build_solidity(
        r##"
        contract c {
            function test(string a) public {
            }
        }"##,
    );

    runtime.constructor(&[]);

    // patch offset to be garbage
    runtime.function_abi_fail(
        "test",
        &[ethabi::Token::String("Hello, World!".to_owned())],
        |x| x[30] = 2,
    );

    // patch offset to overflow
    runtime.function_abi_fail(
        "test",
        &[ethabi::Token::String("Hello, World!".to_owned())],
        |x| {
            x[31] = 0xff;
            x[30] = 0xff;
            x[29] = 0xff;
            x[28] = 0xe0;
        },
    );

    // patch length to be garbage
    runtime.function_abi_fail(
        "test",
        &[ethabi::Token::String("Hello, World!".to_owned())],
        |x| x[62] = 2,
    );

    // patch length to overflow
    runtime.function_abi_fail(
        "test",
        &[ethabi::Token::String("Hello, World!".to_owned())],
        |x| {
            x[63] = 0xff;
            x[62] = 0xff;
            x[61] = 0xff;
            x[60] = 0xe0;
        },
    );
}

#[test]
fn external_call() {
    let mut runtime = build_solidity(
        r##"
        contract b {
            int32 x;

            constructor(int32 a) public {
                x = a;
            }

            function get_x(int32 t) public returns (int32) {
                return x * t;
            }
        }

        contract c {
            b x;
        
            constructor() public {
                x = new b(102);
            }

            function test() public returns (int32) {
                return x.get_x({ t: 10 });
            }
        }"##,
    );

    runtime.constructor(&[]);

    let ret = runtime.function("test", &[]);

    assert_eq!(
        ret,
        vec!(ethabi::Token::Int(ethereum_types::U256::from(1020)))
    );
}

#[test]
fn try_catch() {
    let mut runtime = build_solidity(
        r##"
        contract b {
            int32 x;

            constructor(int32 a) public {
                x = a;
            }

            function get_x(int32 t) public returns (int32) {
                if (t == 0) {
                    revert("cannot be zero");
                }
                return x * t;
            }
        }

        contract c {
            b x;
        
            constructor() public {
                x = new b(102);
            }

            function test() public returns (int32) {
                int32 state = 0;
                try x.get_x(0) returns (int32 l) {
                    state = 1;
                } catch Error(string err) {
                    if (err == "cannot be zero") {
                        state = 2;
                    } else {
                        state = 3;
                    }
                } catch (bytes ) {
                    state = 4;
                }

                return state;
            }
        }"##,
    );

    runtime.constructor(&[]);

    let ret = runtime.function("test", &[]);

    assert_eq!(ret, vec!(ethabi::Token::Int(ethereum_types::U256::from(2))));
}

#[test]
fn payables() {
    // no contructors means can't send value
    let mut runtime = build_solidity(
        r##"
        contract c {
            function test(string a) public {
            }
        }"##,
    );

    runtime.value = 1;
    runtime.constructor_expect_revert(&[]);

    // contructors w/o payable means can't send value
    let mut runtime = build_solidity(
        r##"
        contract c {
            constructor() public {
                int32 a = 0;
            }

            function test(string a) public {
            }
        }"##,
    );

    runtime.value = 1;
    runtime.constructor_expect_revert(&[]);

    // contructors w/ payable means can send value
    let mut runtime = build_solidity(
        r##"
        contract c {
            constructor() public payable {
                int32 a = 0;
            }

            function test(string a) public {
            }
        }"##,
    );

    runtime.value = 1;
    runtime.constructor(&[]);

    // function w/o payable means can't send value
    let mut runtime = build_solidity(
        r##"
        contract c {
            function test() public {
            }
        }"##,
    );

    runtime.constructor(&[]);
    runtime.value = 1;
    runtime.function_revert("test", &[]);

    // test both
    let mut runtime = build_solidity(
        r##"
        contract c {
            function test() payable public {
            }
            function test2() public {
            }
        }"##,
    );

    runtime.constructor(&[]);
    runtime.value = 1;
    runtime.function_revert("test2", &[]);
    runtime.value = 1;
    runtime.function("test", &[]);
}

#[test]
fn balance() {
    let mut runtime = build_solidity(
        r##"
        contract c {
            function test() public returns (uint128) {
                return address(this).balance;
            }
        }"##,
    );

    runtime.constructor(&[]);

    runtime.accounts.get_mut(&runtime.vm.cur).unwrap().1 = 512;
    let ret = runtime.function("test", &[]);

    assert_eq!(
        ret,
        vec!(ethabi::Token::Uint(ethereum_types::U256::from(512)))
    );
}

#[test]
fn selfdestruct() {
    let mut runtime = build_solidity(
        r##"
        contract other {
            function goaway(address payable recipient) public returns (bool) {
                selfdestruct(recipient);
            }
        }

        contract c {
            other o;
            function step1() public {
                o = new other{value: 511}();
            }

            function step2() public {
                o.goaway(payable(address(this)));
            }
        }"##,
    );

    runtime.constructor(&[]);

    runtime.function("step1", &[]);
    runtime.accounts.get_mut(&runtime.vm.cur).unwrap().1 = 0;

    runtime.function("step2", &[]);
    runtime.accounts.get_mut(&runtime.vm.cur).unwrap().1 = 511;
}
