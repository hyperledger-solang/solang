
#[cfg(test)]
mod tests {
    use solidity;
    use resolve;
    use emit::Emitter;
    use wasmi::{ImportsBuilder, Module, ModuleInstance, NopExternals, RuntimeValue};

    #[test]
    fn simple_solidiy_compile_and_run() {
        // parse
        let mut s = solidity::SourceUnitParser::new()
            .parse("
            contract test {
                function foo() returns (uint32) {
                    return 2;
                }
            }").expect("parse should succeed");
        
        // resolve
        resolve::resolve(&mut s).expect("resolve should succeed");

        // codegen
        Emitter::init();

        let res = Emitter::new(s);

        assert_eq!(res.contracts.len(), 1);

        let bc = res.contracts[0].wasm(&res).expect("llvm wasm emit should work");

        let module = Module::from_buffer(bc).expect("parse wasm should work");

        let main = ModuleInstance::new(&module, &ImportsBuilder::default())
            .expect("Failed to instantiate module")
            .run_start(&mut NopExternals)
            .expect("Failed to run start function in module");

        let ret = main.invoke_export("foo", &[], &mut NopExternals).expect("failed to call function");

        assert_eq!(ret, Some(RuntimeValue::I32(2)));
    }
}