// Create WASM virtual machine like substrate

use abi;
use emit;
use link;
use output;
use parser;
use resolver;
use std::collections::HashMap;
use wasmi::memory_units::Pages;
use wasmi::*;

use parity_scale_codec::Encode;
use parity_scale_codec_derive::{Encode, Decode};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

type StorageKey = [u8; 32];

#[derive(FromPrimitive)]
#[allow(non_camel_case_types)]
enum SubstrateExternal {
    ext_scratch_size = 0,
    ext_scratch_read,
    ext_scratch_write,
    ext_set_storage,
    ext_get_storage,
    ext_return
}

struct ContractStorage {
    memory: MemoryRef,
    scratch: Vec<u8>,
    store: HashMap<StorageKey, Vec<u8>>,
}

impl ContractStorage {
    fn new() -> Self {
        ContractStorage {
            memory: MemoryInstance::alloc(Pages(2), Some(Pages(2))).unwrap(),
            scratch: Vec::new(),
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

        match FromPrimitive::from_usize(index) {
            Some(SubstrateExternal::ext_scratch_size) => {
                Ok(Some(RuntimeValue::I32(self.scratch.len() as i32)))
            },
            Some(SubstrateExternal::ext_scratch_read) => {
                let dest: u32 = args.nth_checked(0)?;
                let offset: u32 = args.nth_checked(1)?;
                let len: u32 = args.nth_checked(2)?;

                if let Err(e) = self.memory.set(dest, &self.scratch[offset as usize..(offset + len) as usize]) {
                    panic!("ext_set_storage: {}", e);
                }

                Ok(None)
            },
            Some(SubstrateExternal::ext_scratch_write) => {
                let dest: u32 = args.nth_checked(0)?;
                let len: u32 = args.nth_checked(1)?;

                self.scratch.resize(len as usize, 0u8);

                if let Err(e) = self.memory.get_into(dest, &mut self.scratch) {
                    panic!("ext_set_storage: {}", e);
                }

                Ok(None)
            },
            Some(SubstrateExternal::ext_get_storage) => {
                let key_ptr: u32 = args.nth_checked(0)?;

                let mut key: StorageKey = [0; 32];

                if let Err(e) = self.memory.get_into(key_ptr, &mut key) {
                    panic!("ext_set_storage: {}", e);
                }

                if self.store.contains_key(&key) {
                    self.scratch = self.store[&key].clone();
                    Ok(Some(RuntimeValue::I32(0)))
                } else {
                    self.scratch.clear();
                    Ok(Some(RuntimeValue::I32(1)))
                }
            },
            Some(SubstrateExternal::ext_set_storage) => {
                let key_ptr: u32 = args.nth_checked(0)?;
                let value_non_null: u32 = args.nth_checked(1)?;
                let data_ptr: u32 = args.nth_checked(2)?;
                let len: u32 = args.nth_checked(3)?;

                let mut key: StorageKey = [0; 32];

                if let Err(e) = self.memory.get_into(key_ptr, &mut key) {
                    panic!("ext_set_storage: {}", e);
                }

                if value_non_null != 0 {
                    let mut data = Vec::new();
                    data.resize(len as usize, 0u8);

                    if let Err(e) = self.memory.get_into(data_ptr, &mut data) {
                        panic!("ext_set_storage: {}", e);
                    }
                    self.store.insert(key, data);
                } else {
                    self.store.remove(&key);
                }

                Ok(None)
            },
            Some(SubstrateExternal::ext_return) => {
                let data_ptr: u32 = args.nth_checked(0)?;
                let len: u32 = args.nth_checked(1)?;

                self.scratch.resize(len as usize, 0u8);

                if let Err(e) = self.memory.get_into(data_ptr, &mut self.scratch) {
                    panic!("ext_set_storage: {}", e);
                }

                Ok(None)
            },
            _ => panic!("external {} unknown", index),
        }
    }
}

impl ModuleImportResolver for ContractStorage {
    fn resolve_func(&self, field_name: &str, signature: &Signature) -> Result<FuncRef, Error> {
        let index = match field_name {
            "ext_scratch_size" => SubstrateExternal::ext_scratch_size,
            "ext_scratch_read" => SubstrateExternal::ext_scratch_read,
            "ext_scratch_write" => SubstrateExternal::ext_scratch_write,
            "ext_get_storage" => SubstrateExternal::ext_get_storage,
            "ext_set_storage" => SubstrateExternal::ext_set_storage,
            "ext_return" => SubstrateExternal::ext_return,
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
    abi: abi::substrate::Metadata
}

impl TestRuntime {
    fn function<'a>(&self, store: &mut ContractStorage, name: &str, args: Vec<u8>) {
        let m = self.abi.get_function(name).unwrap();

        store.scratch = m.selector.to_le_bytes().to_vec().into_iter().chain(args).collect();

        self.module
            .invoke_export("call", &[], store)
            .expect("failed to call function");
    }
}

fn build_solidity(ctx: &inkwell::context::Context, src: &'static str) -> (TestRuntime, ContractStorage) {
    let s = parser::parse(src).expect("parse should succeed");

    // resolve
    let (contracts, errors) = resolver::resolver(s, &resolver::Target::Substrate);

    if contracts.is_empty() {
        output::print_messages("test.sol", src, &errors, false);
    }

    assert_eq!(contracts.len(), 1);

    // abi
    let (abistr, _) = abi::generate_abi(&contracts[0], false);

    // codegen
    let contract = emit::Contract::build(ctx, &contracts[0], &"foo.sol");

    let obj = contract.wasm("default").expect("llvm wasm emit should work");

    let bc = link::link(&obj, &resolver::Target::Substrate);

    let module = Module::from_buffer(bc).expect("parse wasm should work");

    let store = ContractStorage::new();

    (
        TestRuntime{
            module: ModuleInstance::new(&module, &ImportsBuilder::new().with_resolver("env", &store))
                .expect("Failed to instantiate module")
                .run_start(&mut NopExternals)
                .expect("Failed to run start function in module"),
            abi: abi::substrate::load(&abistr).unwrap()
        },
        store
    )
}


#[test]
fn simple_solidiy_compile_and_run() {
    let ctx = inkwell::context::Context::create();

    #[derive(Debug, PartialEq, Encode, Decode)]
    struct FooReturn {
        value: u32
    }

    // parse
    let (runtime, mut store) = build_solidity(&ctx,
        "
        contract test {
            function foo() public returns (uint32) {
                return 2;
            }
        }",
    );

    runtime.function(&mut store, "foo", Vec::new());

    let ret = FooReturn{ value: 2 };

    assert_eq!(store.scratch, ret.encode());
}

#[test]
fn flipper() {
    let ctx = inkwell::context::Context::create();

    // parse
    let (runtime, mut store) = build_solidity(&ctx,
        "
        contract flipper {
            bool private value;

            constructor(bool initvalue) public {
                value = initvalue;
            }

            function flip() public {
                value = !value;
            }

            function get() public view returns (bool) {
                return value;
            }
        }
        ",
    );

    #[derive(Debug, PartialEq, Encode, Decode)]
    struct GetReturn(bool);

    runtime.function(&mut store, "get", Vec::new());

    assert_eq!(store.scratch, GetReturn(false).encode());

    runtime.function(&mut store, "flip", Vec::new());
    runtime.function(&mut store, "flip", Vec::new());
    runtime.function(&mut store, "flip", Vec::new());

    runtime.function(&mut store, "get", Vec::new());

    assert_eq!(store.scratch, GetReturn(true).encode());
}