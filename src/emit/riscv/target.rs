use inkwell::{
    values::{BasicValueEnum, IntValue},
    InlineAsmDialect,
};

/// Emit `ecall` with syscall id in t0, up to 4 u64 args in a0..a3
/// (r55's SLoad/SStore key width),
/// return value(s) collected from a0..a3 depending on `n_returns`.
fn emit_ecall<'a>(
    bin: &Binary<'a>,
    syscall_id: u64,
    args: &[IntValue<'a>],
    n_returns: usize,
) -> Vec<IntValue<'a>> {
    let i64_ty = bin.context.i64_type();
    // simplistic; adjust per n_returns
    let fn_ty = i64_ty.fn_type(&vec![i64_ty.into(); args.len()], false);

    // Constraint string mirrors r55's convention: t0=syscall id (fixed via "+{t0}"),
    // a0..a3 = args/rets. Exact inkwell constraint syntax needs verification against
    // your inkwell version's InlineAsm API -- check docs.rs for the version pinned
    // in Cargo.toml, this sketch is illustrative, not copy-paste-safe.
    let asm = bin.context.create_inline_asm(
        fn_ty,
        "ecall".to_string(),
        "={a0},={a1},={a2},={a3},{a0},{a1},{a2},{a3},{t0}".to_string(),
        true,  // has side effects
        false, // not align stack
        Some(InlineAsmDialect::Att),
        false,
    );

    // build_indirect_call with `asm` as the callee,
    // args + syscall_id constant as operands
    todo!("wire up build_indirect_call with asm value, args, and t0 constant")
}
