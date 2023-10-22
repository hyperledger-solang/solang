use std::ffi::OsStr;

use solang::{
    codegen::codegen,
    file_resolver::FileResolver,
    parse_and_resolve,
    sema::ast::Namespace,
    ssa_ir::{converter::Converter, printer::Printer},
    stringfy_cfg, Target,
};

fn new_file_resolver(src: &str) -> FileResolver {
    let mut cache = FileResolver::default();
    cache.set_file_contents("test.sol", src.to_string());
    cache
}

#[test]
fn test_convert_cfg() {
    let src = r#"
contract dynamicarray {
    function test() public pure {
        int64[] memory a = new int64[](3);
        a[0] = 1;
        a[1] = 2;
        a[2] = 3;
        a.push(4);

        assert(a.length == 4);
    }
}"#;

    let mut resolver = new_file_resolver(src);
    let mut ns: Namespace =
        parse_and_resolve(OsStr::new("test.sol"), &mut resolver, Target::Solana);
    // check errors
    if !ns.diagnostics.is_empty() {
        ns.print_diagnostics_in_plain(&resolver, true);
        // panic!("compile error");
    }
    codegen(&mut ns, &Default::default());
    let contract = ns.contracts.get(0).unwrap();
    let cfg = contract.cfg.get(0).unwrap();
    let str = &cfg.to_string(&contract, &ns);
    println!("=====================");
    println!("cfg: {}", str);
    println!("=====================");

    // let stmt = cfg.blocks.get(5).unwrap().instr.get(2).unwrap();
    // match stmt {
    //     Instr::PushMemory {
    //         res,
    //         ty,
    //         array,
    //         value,
    //     } => {
    //         println!("res: {:?}", cfg.vars.get(res).unwrap());
    //         println!("ty: {:?}", ty);
    //         println!("array: {:?}", cfg.vars.get(array).unwrap());
    //         println!("value: {:?}", value);
    //     }
    //     _ => panic!(),
    // }

    let converter = Converter::new(&ns, cfg);
    let new_cfg = converter.get_ssa_ir_cfg().unwrap();

    let printer = Printer {
        vartable: Box::new(new_cfg.vartable.clone()),
    };

    let expected_cfg_str = r#"public function sol#2 dynamicarray::dynamicarray::function::test ():
block#0 entry:
    uint32 %array_length.temp.1 = 3;
    uint32 %temp.ssa_ir.9 = 3;
    ptr<int64[]> %a = alloc int64[][uint32(%temp.ssa_ir.9)];
    uint32 %index.temp.3 = 0;
    uint32 %temp.ssa_ir.11 = 0;
    uint32 %temp.ssa_ir.12 = 3;
    bool %temp.ssa_ir.10 = uint32(%temp.ssa_ir.11) (u)>= uint32(%temp.ssa_ir.12);
    cbr bool(%temp.ssa_ir.10) block#1 else block#2;

block#1 out_of_bounds:
    assert_failure;

block#2 in_bounds:
    int64 %temp.2 = 1;
    ptr<int64[]> %temp.ssa_ir.14 = ptr<int64[]>(%a);
    uint32 %temp.ssa_ir.15 = 0;
    ptr<int64> %temp.ssa_ir.13 = ptr<int64[]>(%temp.ssa_ir.14)[uint32(%temp.ssa_ir.15)];
    int64 %temp.ssa_ir.16 = 1;
    store int64(%temp.ssa_ir.16) to ptr<int64>(%temp.ssa_ir.13);
    uint32 %index.temp.5 = 1;
    uint32 %temp.ssa_ir.18 = 1;
    uint32 %temp.ssa_ir.19 = 3;
    bool %temp.ssa_ir.17 = uint32(%temp.ssa_ir.18) (u)>= uint32(%temp.ssa_ir.19);
    cbr bool(%temp.ssa_ir.17) block#3 else block#4;

block#3 out_of_bounds:
    assert_failure;

block#4 in_bounds:
    int64 %temp.4 = 2;
    ptr<int64[]> %temp.ssa_ir.21 = ptr<int64[]>(%a);
    uint32 %temp.ssa_ir.22 = 1;
    ptr<int64> %temp.ssa_ir.20 = ptr<int64[]>(%temp.ssa_ir.21)[uint32(%temp.ssa_ir.22)];
    int64 %temp.ssa_ir.23 = 2;
    store int64(%temp.ssa_ir.23) to ptr<int64>(%temp.ssa_ir.20);
    uint32 %index.temp.7 = 2;
    uint32 %temp.ssa_ir.25 = 2;
    uint32 %temp.ssa_ir.26 = 3;
    bool %temp.ssa_ir.24 = uint32(%temp.ssa_ir.25) (u)>= uint32(%temp.ssa_ir.26);
    cbr bool(%temp.ssa_ir.24) block#5 else block#6;

block#5 out_of_bounds:
    assert_failure;

block#6 in_bounds:
    int64 %temp.6 = 3;
    ptr<int64[]> %temp.ssa_ir.28 = ptr<int64[]>(%a);
    uint32 %temp.ssa_ir.29 = 2;
    ptr<int64> %temp.ssa_ir.27 = ptr<int64[]>(%temp.ssa_ir.28)[uint32(%temp.ssa_ir.29)];
    int64 %temp.ssa_ir.30 = 3;
    store int64(%temp.ssa_ir.30) to ptr<int64>(%temp.ssa_ir.27);
    int64 %temp.8 = 4;
    int64 %temp.8 = push_mem ptr<int64[]>(%a) int64(%temp.8);
    uint32 %array_length.temp.1 = 4;
    uint32 %temp.ssa_ir.32 = 4;
    uint32 %temp.ssa_ir.33 = 4;
    bool %temp.ssa_ir.31 = uint32(%temp.ssa_ir.32) == uint32(%temp.ssa_ir.33);
    cbr bool(%temp.ssa_ir.31) block#7 else block#8;

block#7 noassert:
    return;

block#8 doassert:
    assert_failure;

"#;

    assert_eq!(stringfy_cfg!(printer, &new_cfg), expected_cfg_str);

    // use '%temp\.ssa_ir\.\d+ =' to get all the temp variables in the cfg and check if they are duplicated
    let re = regex::Regex::new(r"%temp\.ssa_ir\.\d+ =").unwrap();
    let mut temp_vars = Vec::new();
    for cap in re.captures_iter(expected_cfg_str) {
        temp_vars.push(cap[0].to_string());
    }
    // check if there are duplicated temp variables
    let mut temp_vars_clone = temp_vars.clone();
    temp_vars_clone.dedup();
    
    // assert length equal
    assert_eq!(temp_vars.len(), temp_vars_clone.len());
}
