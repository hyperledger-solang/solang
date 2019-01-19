
#[cfg(test)]
mod tests {
    use parse;
    use resolve;
    use emit::Emitter;
    use wasmi::{ImportsBuilder, Module, ModuleInstance, NopExternals, RuntimeValue, ModuleRef};

    fn build_solidity(src: &'static str) -> ModuleRef {
        let mut s = parse::parse(src).expect("parse should succeed");
        
        // resolve
        resolve::resolve(&mut s);

        // codegen
        let res = Emitter::new(s);

        assert_eq!(res.contracts.len(), 1);

        let bc = res.contracts[0].wasm(&res).expect("llvm wasm emit should work");

        let module = Module::from_buffer(bc).expect("parse wasm should work");

        ModuleInstance::new(&module, &ImportsBuilder::default())
            .expect("Failed to instantiate module")
            .run_start(&mut NopExternals)
            .expect("Failed to run start function in module")
    }

    #[test]
    fn simple_solidiy_compile_and_run() {
        // parse
        let main = build_solidity("
            contract test {
                function foo() returns (uint32) {
                    return 2;
                }
            }");

        let ret = main.invoke_export("foo", &[], &mut NopExternals).expect("failed to call function");

        assert_eq!(ret, Some(RuntimeValue::I32(2)));
    }

    #[test]
    fn simple_loops() {
        let main = build_solidity(r##"
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

            let ret = main.invoke_export("foo", &[RuntimeValue::I32(i)], &mut NopExternals).expect("failed to call function");

            assert_eq!(ret, Some(RuntimeValue::I32(res)));
        }

        for i in 0..=50 {
            let res = (i + 1) * 10 + 1;

            let ret = main.invoke_export("bar", &[RuntimeValue::I32(i), RuntimeValue::I32(1)], &mut NopExternals).expect("failed to call function");

            assert_eq!(ret, Some(RuntimeValue::I32(res)));
        }

        for i in 0..=50 {
            let mut res = 1;

            for _ in 2..10 {
                res *= 3;
            }

            let ret = main.invoke_export("bar", &[RuntimeValue::I32(i), RuntimeValue::I32(0)], &mut NopExternals).expect("failed to call function");

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

            let ret = main.invoke_export("baz", &[RuntimeValue::I32(i)], &mut NopExternals).expect("failed to call function");

            assert_eq!(ret, Some(RuntimeValue::I32(res)));
        }

    }
}