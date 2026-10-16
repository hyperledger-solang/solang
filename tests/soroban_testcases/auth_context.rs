// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use soroban_sdk::{
    auth::{
        Context, ContractContext, ContractExecutable, CreateContractHostFnContext,
        CreateContractWithConstructorHostFnContext,
    },
    testutils::{Address as _, BytesN as _},
    vec, Address, BytesN, Env, Error, FromVal, IntoVal, String, Symbol, Val,
};

/// Records the last context passed to `__check_auth`, so the test can read it back.
const CONTRACT: &str = r#"
contract probe {
    uint32 public count;
    uint32 public kind;
    address public target;
    string public fn_name;
    bytes32 public wasm_hash;
    int128 public amount;

    function __check_auth(bytes32 payload, bytes signature, Context[] auth_contexts) public {
        count = auth_contexts.length;
        Context memory ctx = auth_contexts[auth_contexts.length - 1];
        kind = uint32(ctx.kind);
        wasm_hash = ctx.wasm_hash;
        if (ctx.kind == ContextKind.Contract) {
            target = ctx.contract_address;
            fn_name = ctx.fn_name;
        }
        if (ctx.kind != ContextKind.CreateContract) {
            (address from, address to, int128 value) = abi.decode(ctx.args, (address, address, int128));
            amount = value;
        }
    }
}
"#;

fn get<T: FromVal<Env, Val>>(env: &Env, addr: &Address, name: &str) -> T {
    T::from_val(
        env,
        &env.invoke_contract::<Val>(addr, &Symbol::new(env, name), vec![env]),
    )
}

fn check_auth(env: &Env, addr: &Address, contexts: soroban_sdk::Vec<Context>) {
    let payload = BytesN::<32>::random(env);
    let signature: Val = BytesN::<64>::random(env).into_val(env);
    env.try_invoke_contract_check_auth::<Error>(addr, &payload, signature, &contexts)
        .unwrap();
}

#[test]
fn auth_context_kinds_are_decoded() {
    let runtime = build_solidity(CONTRACT, |_| {});
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;
    let (token, from, to) = (
        Address::generate(env),
        Address::generate(env),
        Address::generate(env),
    );
    let transfer_args = vec![
        env,
        from.into_val(env),
        to.into_val(env),
        1000_i128.into_val(env),
    ];
    let hash = BytesN::<32>::random(env);

    // Contract call, with a short and a long (non-inline) function name
    for name in ["transfer", "transfer_from_a_long_name"] {
        let contract_ctx = Context::Contract(ContractContext {
            contract: token.clone(),
            fn_name: Symbol::new(env, name),
            args: transfer_args.clone(),
        });
        check_auth(env, addr, vec![env, contract_ctx]);
        assert_eq!(get::<u32>(env, addr, "kind"), 0);
        assert_eq!(get::<Address>(env, addr, "target"), token);
        assert_eq!(
            get::<String>(env, addr, "fn_name"),
            String::from_str(env, name)
        );
        assert_eq!(get::<i128>(env, addr, "amount"), 1000);
    }

    // Contract creation, with and without constructor arguments; several contexts at once
    let create = Context::CreateContractHostFn(CreateContractHostFnContext {
        executable: ContractExecutable::Wasm(hash.clone()),
        salt: BytesN::random(env),
    });
    check_auth(env, addr, vec![env, create.clone()]);
    assert_eq!(get::<u32>(env, addr, "kind"), 1);
    assert_eq!(get::<BytesN<32>>(env, addr, "wasm_hash"), hash);

    let create_with_ctor =
        Context::CreateContractWithCtorHostFn(CreateContractWithConstructorHostFnContext {
            executable: ContractExecutable::Wasm(hash.clone()),
            salt: BytesN::random(env),
            constructor_args: vec![
                env,
                from.into_val(env),
                to.into_val(env),
                7_i128.into_val(env),
            ],
        });
    check_auth(env, addr, vec![env, create, create_with_ctor]);
    assert_eq!(get::<u32>(env, addr, "count"), 2);
    assert_eq!(get::<u32>(env, addr, "kind"), 2);
    assert_eq!(get::<BytesN<32>>(env, addr, "wasm_hash"), hash);
    assert_eq!(get::<i128>(env, addr, "amount"), 7);
}
