
#[cfg(test)]
mod tests {
    use parser;
    use resolver;
    use emit;
    use link;
    use wasmi::{ImportsBuilder, Module, ModuleInstance, NopExternals, RuntimeValue, ModuleRef};
    use std::mem;

    fn build_solidity(src: &'static str) -> (ModuleRef, String) {
        let s = parser::parse(src).expect("parse should succeed");
        
        // resolve
        let (contracts, _errors) = resolver::resolver(s);

        assert_eq!(contracts.len(), 1);

        // abi
        let abi = contracts[0].generate_abi();

        // codegen
        let contract = emit::Contract::new(&contracts[0], &"foo.sol");

        let obj = contract.wasm().expect("llvm wasm emit should work");

        let bc = link::link(&obj);

        let module = Module::from_buffer(bc).expect("parse wasm should work");

        (ModuleInstance::new(&module, &ImportsBuilder::default())
            .expect("Failed to instantiate module")
            .run_start(&mut NopExternals)
            .expect("Failed to run start function in module"),
         serde_json::to_string(&abi).unwrap())
    }

    #[test]
    fn simple_solidiy_compile_and_run() {
        // parse
        let (main, _) = build_solidity("
            contract test {
                function foo() returns (uint32) {
                    return 2;
                }
            }");

        let ret = main.invoke_export("sol::foo", &[], &mut NopExternals).expect("failed to call function");

        assert_eq!(ret, Some(RuntimeValue::I32(2)));
    }

    #[test]
    fn simple_loops() {
        let (main, _) = build_solidity(r##"
contract test3 {
	function foo(uint32 a) returns (uint32) {
		uint32 b = 50 - a;
		uint32 c;
		c = 100 * b;
		c += 5;
		return a * 1000 + c;
	}

	function bar(uint32 b, bool x) returns (uint32) {
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

	function baz(uint32 x) returns (uint32) {
		for (uint32 i = 0; i<100; i++) {
			x *= 7;

			if (x > 200) {
				break;
			}

			x++;
		}

		return x;
	}
}"##);

        for i in 0..=50 {
            let res = ((50 - i) * 100 + 5) + i * 1000;

            let ret = main.invoke_export("sol::foo", &[RuntimeValue::I32(i)], &mut NopExternals).expect("failed to call function");

            assert_eq!(ret, Some(RuntimeValue::I32(res)));
        }

        for i in 0..=50 {
            let res = (i + 1) * 10 + 1;

            let ret = main.invoke_export("sol::bar", &[RuntimeValue::I32(i), RuntimeValue::I32(1)], &mut NopExternals).expect("failed to call function");

            assert_eq!(ret, Some(RuntimeValue::I32(res)));
        }

        for i in 0..=50 {
            let mut res = 1;

            for _ in 2..10 {
                res *= 3;
            }

            let ret = main.invoke_export("sol::bar", &[RuntimeValue::I32(i), RuntimeValue::I32(0)], &mut NopExternals).expect("failed to call function");

            assert_eq!(ret, Some(RuntimeValue::I32(res)));
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

            let ret = main.invoke_export("sol::baz", &[RuntimeValue::I32(i)], &mut NopExternals).expect("failed to call function");

            assert_eq!(ret, Some(RuntimeValue::I32(res)));
        }
    }

    #[test]
    fn stack_test() {
        let (main, _) = build_solidity(r##"
contract test3 {
	function foo() returns (bool) {
		uint b = 18446744073709551616;
        uint c = 36893488147419103232;

        return b * 2 == c;
	}
}"##);

        let ret = main.invoke_export("sol::foo", &[], &mut NopExternals).expect("failed to call function");

        assert_eq!(ret, Some(RuntimeValue::I32(1)));
    }

    #[test]
    fn abi_call_return_test() {
        let (wasm, abi) = build_solidity(r##"
contract test {
	function foo() returns (uint32) {
        return 102;
	}
}"##);

        let abi = ethabi::Contract::load(abi.as_bytes()).unwrap();

        // call constructor so that heap is initialised
        let ret = wasm.invoke_export("constructor", &[RuntimeValue::I32(0)], &mut NopExternals).expect("failed to call constructor");

        assert_eq!(ret, None);

        // create call for foo
        let calldata = abi.functions["foo"].encode_input(&[]).unwrap();
        // need to prepend length
        let wmem = match wasm.export_by_name("memory") {
            Some(wasmi::ExternVal::Memory(n)) => n,
            _ => panic!()
        };

        wmem.set_value(0, calldata.len() as u32).unwrap();
        wmem.set(mem::size_of::<u32>() as  u32, &calldata).unwrap();

        let ret = wasm.invoke_export("function", &[RuntimeValue::I32(0)], &mut NopExternals).expect("failed to call function");

        if let Some(RuntimeValue::I32(offset)) = ret {
            let offset = offset as u32;
            assert_eq!(wmem.get_value::<u32>(offset).unwrap(), 32);
            let returndata = wmem.get(offset + mem::size_of::<u32>() as u32, 32).unwrap();

            let returns = abi.functions["foo"].decode_output(&returndata).unwrap();

            assert_eq!(returns, vec![ ethabi::Token::Uint(ethereum_types::U256::from_dec_str("102").unwrap()) ]);
        } else {
            panic!("expected offset to return data");
        }
    }

    #[test]
    fn abi_call_pass_return_test() {
        let (wasm, abi) = build_solidity(r##"
contract test {
	function foo(uint32 a) returns (uint32) {
        return a;
	}
}"##);

        let abi = ethabi::Contract::load(abi.as_bytes()).unwrap();

        // call constructor so that heap is initialised
        let ret = wasm.invoke_export("constructor", &[RuntimeValue::I32(0)], &mut NopExternals).expect("failed to call constructor");

        assert_eq!(ret, None);

        for val in [102i32, 255, 256, 0x7fffffff].iter() {
            let eval = ethereum_types::U256::from(*val);
            // create call for foo
            let calldata = abi.functions["foo"].encode_input(&[ ethabi::Token::Uint(eval) ]).unwrap();
            // need to prepend length
            let wmem = match wasm.export_by_name("memory") {
                Some(wasmi::ExternVal::Memory(n)) => n,
                _ => panic!()
            };

            wmem.set_value(0, calldata.len() as u32).unwrap();
            wmem.set(mem::size_of::<u32>() as  u32, &calldata).unwrap();

            let ret = wasm.invoke_export("function", &[RuntimeValue::I32(0)], &mut NopExternals).expect("failed to call function");

            if let Some(RuntimeValue::I32(offset)) = ret {
                let offset = offset as u32;
                assert_eq!(wmem.get_value::<u32>(offset).unwrap(), 32);
                let returndata = wmem.get(offset + mem::size_of::<u32>() as u32, 32).unwrap();

                let returns = abi.functions["foo"].decode_output(&returndata).unwrap();

                assert_eq!(returns, vec![ ethabi::Token::Uint(eval) ]);
            } else {
                panic!("expected offset to return data");
            }
        }
    }
}
