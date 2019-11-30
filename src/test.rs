#[cfg(test)]
mod tests {
    use abi;
    use emit;
    use link;
    use output;
    use parser;
    use resolver;
    use std::collections::HashMap;
    use std::mem;
    use wasmi::memory_units::Pages;
    use wasmi::*;
    use ethabi::Token;

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
        abi: ethabi::Contract
    }

    impl TestRuntime {
        fn function(&self, store: &mut ContractStorage, name: &str, args: &[Token]) -> Vec<Token> {
            let calldata = self.abi.functions[name].encode_input(args).unwrap();
            // need to prepend length
            store.memory.set_value(0, calldata.len() as u32).unwrap();
            store.memory.set(mem::size_of::<u32>() as u32, &calldata).unwrap();
    
            let ret = self.module
                .invoke_export("function", &[RuntimeValue::I32(0)], store)
                .expect("failed to call function");

            match ret {
                Some(RuntimeValue::I32(offset)) => {
                    let offset = offset as u32;
                    let returndata = store.memory.get(offset + mem::size_of::<u32>() as u32, 32).unwrap();
        
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
                store.memory.set(mem::size_of::<u32>() as u32, &calldata).unwrap();
        
                let ret = self.module
                    .invoke_export("constructor", &[RuntimeValue::I32(0)], store)
                    .expect("failed to call constructor");

                match ret {
                    None => (),
                    _ => panic!("not expected return value when calling constructor"),
                }
            }
        }
    }

    fn build_solidity(ctx: &inkwell::context::Context, src: &'static str) -> (TestRuntime, ContractStorage) {
        let s = parser::parse(src).expect("parse should succeed");

        // resolve
        let (contracts, errors) = resolver::resolver(s, &resolver::Target::Burrow);

        if contracts.is_empty() {
            output::print_messages("test.sol", src, &errors, false);
        }

        assert_eq!(contracts.len(), 1);

        // abi
        let abi = abi::ethabi::gen_abi(&contracts[0]);

        // codegen
        let contract = emit::Contract::build(ctx, &contracts[0], &"foo.sol");

        let obj = contract.wasm("default").expect("llvm wasm emit should work");

        let bc = link::link(&obj, &resolver::Target::Burrow);

        let module = Module::from_buffer(bc).expect("parse wasm should work");

        let store = ContractStorage::new();

        let abi = serde_json::to_string(&abi).unwrap();
        
        (
            TestRuntime{
                module: ModuleInstance::new(&module, &ImportsBuilder::new().with_resolver("env", &store))
                    .expect("Failed to instantiate module")
                    .run_start(&mut NopExternals)
                    .expect("Failed to run start function in module"),
                abi: ethabi::Contract::load(abi.as_bytes()).unwrap(),
            },
            store
        )
    }

    #[test]
    fn simple_solidiy_compile_and_run() {
        let ctx = inkwell::context::Context::create();

        // parse
        let (runtime, mut store) = build_solidity(&ctx,
            "
            contract test {
                function foo() public returns (uint32) {
                    return 2;
                }
            }",
        );

        let returns = runtime.function(&mut store, "foo", &[]);

        assert_eq!(returns, vec![ethabi::Token::Uint(ethereum_types::U256::from(2))]);
    }

    #[test]
    fn simple_loops() {
        let ctx = inkwell::context::Context::create();

        let (runtime, mut store) = build_solidity(&ctx,
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

            let returns = runtime.function(&mut store, "foo", &[ethabi::Token::Uint(ethereum_types::U256::from(i))]);

            assert_eq!(returns, vec![ethabi::Token::Uint(ethereum_types::U256::from(res))]);
        }

        for i in 0..=50 {
            let res = (i + 1) * 10 + 1;

            let returns = runtime.function(&mut store, "bar", 
                &[
                    ethabi::Token::Uint(ethereum_types::U256::from(i)),
                    ethabi::Token::Bool(true)
                ]);

            assert_eq!(returns, vec![ethabi::Token::Uint(ethereum_types::U256::from(res))]);
        }

        for i in 0..=50 {
            let mut res = 1;

            for _ in 2..10 {
                res *= 3;
            }

            let returns = runtime.function(&mut store, "bar", 
                &[
                    ethabi::Token::Uint(ethereum_types::U256::from(i)),
                    ethabi::Token::Bool(false)
                ]);

            assert_eq!(returns, vec![ethabi::Token::Uint(ethereum_types::U256::from(res))]);
        }

        for i in 1..=50 {
            let mut res = i;

            for _ in 0..100 {
                res = res * 7;
                if res > 200 {
                    break;
                }
                res += 1;
            }

            let returns = runtime.function(&mut store, "baz", 
                &[
                    ethabi::Token::Uint(ethereum_types::U256::from(i)),
                ]);

            assert_eq!(returns, vec![ethabi::Token::Uint(ethereum_types::U256::from(res))]);
        }
    }

    #[test]
    fn stack_test() {
        let ctx = inkwell::context::Context::create();

        let (runtime, mut store) = build_solidity(&ctx,
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
        let ctx = inkwell::context::Context::create();

        let (runtime, mut store) = build_solidity(&ctx,
            r##"
contract test {
	function foo() public returns (uint32) {
        return 102;
	}
}"##,
        );

        let returns = runtime.function(&mut store, "foo", &[]);

        assert_eq!(returns, vec![ethabi::Token::Uint(ethereum_types::U256::from(102))]);
    }

    #[test]
    fn abi_call_pass_return_test() {
        let ctx = inkwell::context::Context::create();

        let (runtime, mut store) = build_solidity(&ctx,
            r##"
contract test {
	function foo(uint32 a) public returns (uint32) {
        return a;
	}
}"##,
        );

        for val in [102i32, 255, 256, 0x7fffffff].iter() {
            let returns = runtime.function(&mut store, "foo", 
                &[
                    ethabi::Token::Uint(ethereum_types::U256::from(*val)),
                ]);

            assert_eq!(returns, vec![ethabi::Token::Uint(ethereum_types::U256::from(*val))]);
        }
    }

    #[test]
    fn contract_storage_test() {
        let ctx = inkwell::context::Context::create();

        let (runtime, mut store) = build_solidity(&ctx,
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
}
