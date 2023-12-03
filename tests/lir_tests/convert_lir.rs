// SPDX-License-Identifier: Apache-2.0

use std::ffi::OsStr;

use solang::{
    codegen::codegen,
    file_resolver::FileResolver,
    lir::{converter::Converter, printer::Printer},
    parse_and_resolve,
    sema::ast::Namespace,
    Target,
};

use crate::stringfy_lir;

fn new_file_resolver(src: &str) -> FileResolver {
    let mut cache = FileResolver::default();
    cache.set_file_contents("test.sol", src.to_string());
    cache
}

fn assert_lir_str_eq(src: &str, cfg_no: usize, expected: &str) {
    let mut resolver = new_file_resolver(src);
    let mut ns: Namespace =
        parse_and_resolve(OsStr::new("test.sol"), &mut resolver, Target::Solana);
    // check errors
    // if !ns.diagnostics.is_empty() {
    //     ns.print_diagnostics_in_plain(&resolver, true);
    // panic!("compile error");
    // }
    codegen(&mut ns, &Default::default());
    let contract = ns.contracts.get(0).unwrap();
    let cfg = contract.cfg.get(cfg_no).unwrap();

    // let str = &cfg.to_string(&contract, &ns);
    // println!("=====================");
    // println!("cfg: {}", str);
    // println!("=====================cfg no: {}", cfg_no);

    let converter = Converter::new(&ns, cfg);
    let lir = converter.get_lir();

    // let printer = Printer {
    //     vartable: Box::new(new_cfg.vartable.clone()),
    // };
    let printer = Printer::new(&lir.vartable);

    // printer.print_cfg(&mut std::io::stdout(), &new_cfg).unwrap();
    let result = stringfy_lir!(printer, &lir);
    assert_eq!(result.trim(), expected);

    // use '%temp\.ssa_ir\.\d+ =' to get all the temp variables in the cfg and check if they are duplicated
    let re = regex::Regex::new(r"%temp\.ssa_ir\.\d+ =").unwrap();
    let mut temp_vars = Vec::new();
    for cap in re.captures_iter(result.as_str()) {
        temp_vars.push(cap[0].to_string());
    }
    // check if there are duplicated temp variables
    let mut temp_vars_clone = temp_vars.clone();
    temp_vars_clone.dedup();

    // assert length equal
    assert_eq!(temp_vars.len(), temp_vars_clone.len());
}

#[test]
fn test_convert_lir() {
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

    assert_lir_str_eq(
        src,
        0,
        r#"public function sol#2 dynamicarray::dynamicarray::function::test ():
block#0 entry:
    uint32 %array_length.temp.1 = 3;
    ptr<int64[]> %a = alloc ptr<int64[]>[uint32(3)];
    uint32 %index.temp.3 = 0;
    bool %temp.ssa_ir.9 = uint32(0) (u)>= uint32(3);
    cbr bool(%temp.ssa_ir.9) block#1 else block#2;

block#1 out_of_bounds:
    assert_failure;

block#2 in_bounds:
    int64 %temp.2 = 1;
    ptr<int64> %temp.ssa_ir.10 = ptr<int64[]>(%a)[uint32(0)];
    store int64(1) to ptr<int64>(%temp.ssa_ir.10);
    uint32 %index.temp.5 = 1;
    bool %temp.ssa_ir.11 = uint32(1) (u)>= uint32(3);
    cbr bool(%temp.ssa_ir.11) block#3 else block#4;

block#3 out_of_bounds:
    assert_failure;

block#4 in_bounds:
    int64 %temp.4 = 2;
    ptr<int64> %temp.ssa_ir.12 = ptr<int64[]>(%a)[uint32(1)];
    store int64(2) to ptr<int64>(%temp.ssa_ir.12);
    uint32 %index.temp.7 = 2;
    bool %temp.ssa_ir.13 = uint32(2) (u)>= uint32(3);
    cbr bool(%temp.ssa_ir.13) block#5 else block#6;

block#5 out_of_bounds:
    assert_failure;

block#6 in_bounds:
    int64 %temp.6 = 3;
    ptr<int64> %temp.ssa_ir.14 = ptr<int64[]>(%a)[uint32(2)];
    store int64(3) to ptr<int64>(%temp.ssa_ir.14);
    int64 %temp.8 = push_mem ptr<int64[]>(%a) int64(4);
    uint32 %array_length.temp.1 = 4;
    bool %temp.ssa_ir.15 = uint32(4) == uint32(4);
    cbr bool(%temp.ssa_ir.15) block#7 else block#8;

block#7 noassert:
    return;

block#8 doassert:
    assert_failure;"#,
    );
}

#[test]
fn test_bool_exprs() {
    let cfg_no = 1;

    // read the example.sol file
    let src = r#"
		contract test {
			enum State {
				Running,
				Sleeping,
				Waiting,
				Stopped,
				Zombie,
				StateCount
			}
			State state;
			int32 pid;
			int32 constant first_pid = 1;
			constructor(int32 _pid) {
				pid = _pid;
			}
			function is_zombie_reaper() public view returns (bool) {
				return (pid == first_pid && state != State.Zombie);
			}
		}"#;

    assert_lir_str_eq(
        src,
        cfg_no,
        r#"public function sol#3 test::test::function::is_zombie_reaper () returns (bool):
block#0 entry:
    int32 %temp.3 = load_storage uint32(20);
    bool %and.temp.4 = false;
    bool %temp.ssa_ir.6 = int32(%temp.3) == int32(1);
    cbr bool(%temp.ssa_ir.6) block#1 else block#2;

block#1 and_right_side:
    uint8 %temp.5 = load_storage uint32(16);
    bool %and.temp.4 = uint8(%temp.5) != uint8(4);
    br block#2;

block#2 and_end:
    return bool(%and.temp.4);"#,
    )
}

#[test]
fn test_cast() {
    let cfg_no = 0;

    // read the example.sol file
    let src = r#"
		contract test {
			int32 constant first_pid = 1;
			function systemd_pid() public pure returns (uint32) {
				return uint32(first_pid);
			}
		}"#;

    assert_lir_str_eq(
        src,
        cfg_no,
        r#"public function sol#2 test::test::function::systemd_pid () returns (uint32):
block#0 entry:
    uint32 %temp.ssa_ir.1 = (cast int32(1) to uint32);
    return uint32(%temp.ssa_ir.1);"#,
    )
}

#[test]
fn test_arithmetic_exprs() {
    let cfg_no = 0;

    // read the example.sol file
    let src = r#"
		contract test {
			function celcius2fahrenheit(int32 celcius) pure public returns (int32) {
				int32 fahrenheit = celcius * 9 / 5 + 32;
				return fahrenheit;
			}
		}"#;

    assert_lir_str_eq(
        src,
        cfg_no,
        r#"public function sol#2 test::test::function::celcius2fahrenheit__int32 (int32) returns (int32):
block#0 entry:
    int32 %celcius = int32(arg#0);
    int32 %temp.ssa_ir.4 = int32(%celcius) * int32(9);
    int32 %temp.ssa_ir.3 = int32(%temp.ssa_ir.4) / int32(5);
    int32 %fahrenheit = int32(%temp.ssa_ir.3) + int32(32);
    return int32(%fahrenheit);"#,
    )
}

#[test]
fn test_arithmetic_exprs_1() {
    let cfg_no = 0;

    // read the example.sol file
    let src = r#"
		contract test {
			function byte8reverse(bytes8 input) public pure returns (bytes8 out) {
				out = ((input << 56) & hex"ff00_0000_0000_0000") |
						((input << 40) & hex"00ff_0000_0000_0000") |
						((input << 24) & hex"0000_ff00_0000_0000") |
						((input <<  8) & hex"0000_00ff_0000_0000") |
						((input >>  8) & hex"0000_0000_ff00_0000") |
						((input >> 24) & hex"0000_0000_00ff_0000") |
						((input >> 40) & hex"0000_0000_0000_ff00") |
						((input >> 56) & hex"0000_0000_0000_00ff");
			}
		}"#;

    assert_lir_str_eq(
        src,
        cfg_no,
        r#"public function sol#2 test::test::function::byte8reverse__bytes8 (bytes8) returns (bytes8):
block#0 entry:
    bytes8 %input = bytes8(arg#0);
    bytes8 %out = bytes8 hex"00_00_00_00_00_00_00_00";
    bytes8 %temp.ssa_ir.9 = bytes8(%input) << bytes8(56);
    bytes8 %temp.ssa_ir.10 = bytes8 hex"ff_00_00_00_00_00_00_00";
    bytes8 %temp.ssa_ir.8 = bytes8(%temp.ssa_ir.9) & bytes8(%temp.ssa_ir.10);
    bytes8 %temp.ssa_ir.12 = bytes8(%input) << bytes8(40);
    bytes8 %temp.ssa_ir.13 = bytes8 hex"00_ff_00_00_00_00_00_00";
    bytes8 %temp.ssa_ir.11 = bytes8(%temp.ssa_ir.12) & bytes8(%temp.ssa_ir.13);
    bytes8 %temp.ssa_ir.7 = bytes8(%temp.ssa_ir.8) | bytes8(%temp.ssa_ir.11);
    bytes8 %temp.ssa_ir.15 = bytes8(%input) << bytes8(24);
    bytes8 %temp.ssa_ir.16 = bytes8 hex"00_00_ff_00_00_00_00_00";
    bytes8 %temp.ssa_ir.14 = bytes8(%temp.ssa_ir.15) & bytes8(%temp.ssa_ir.16);
    bytes8 %temp.ssa_ir.6 = bytes8(%temp.ssa_ir.7) | bytes8(%temp.ssa_ir.14);
    bytes8 %temp.ssa_ir.18 = bytes8(%input) << bytes8(8);
    bytes8 %temp.ssa_ir.19 = bytes8 hex"00_00_00_ff_00_00_00_00";
    bytes8 %temp.ssa_ir.17 = bytes8(%temp.ssa_ir.18) & bytes8(%temp.ssa_ir.19);
    bytes8 %temp.ssa_ir.5 = bytes8(%temp.ssa_ir.6) | bytes8(%temp.ssa_ir.17);
    bytes8 %temp.ssa_ir.21 = bytes8(%input) (u)>> bytes8(8);
    bytes8 %temp.ssa_ir.22 = bytes8 hex"00_00_00_00_ff_00_00_00";
    bytes8 %temp.ssa_ir.20 = bytes8(%temp.ssa_ir.21) & bytes8(%temp.ssa_ir.22);
    bytes8 %temp.ssa_ir.4 = bytes8(%temp.ssa_ir.5) | bytes8(%temp.ssa_ir.20);
    bytes8 %temp.ssa_ir.24 = bytes8(%input) (u)>> bytes8(24);
    bytes8 %temp.ssa_ir.25 = bytes8 hex"00_00_00_00_00_ff_00_00";
    bytes8 %temp.ssa_ir.23 = bytes8(%temp.ssa_ir.24) & bytes8(%temp.ssa_ir.25);
    bytes8 %temp.ssa_ir.3 = bytes8(%temp.ssa_ir.4) | bytes8(%temp.ssa_ir.23);
    bytes8 %temp.ssa_ir.27 = bytes8(%input) (u)>> bytes8(40);
    bytes8 %temp.ssa_ir.28 = bytes8 hex"00_00_00_00_00_00_ff_00";
    bytes8 %temp.ssa_ir.26 = bytes8(%temp.ssa_ir.27) & bytes8(%temp.ssa_ir.28);
    bytes8 %temp.ssa_ir.2 = bytes8(%temp.ssa_ir.3) | bytes8(%temp.ssa_ir.26);
    bytes8 %temp.ssa_ir.30 = bytes8(%input) (u)>> bytes8(56);
    bytes8 %temp.ssa_ir.31 = bytes8 hex"00_00_00_00_00_00_00_ff";
    bytes8 %temp.ssa_ir.29 = bytes8(%temp.ssa_ir.30) & bytes8(%temp.ssa_ir.31);
    bytes8 %out = bytes8(%temp.ssa_ir.2) | bytes8(%temp.ssa_ir.29);
    return bytes8(%out);"#,
    )
}

#[test]
fn test_for_loop() {
    let cfg_no = 0;

    // read the example.sol file
    let src = r#"
		contract test {
            enum State {
                Running,
                Sleeping,
                Waiting,
                Stopped,
                Zombie,
                StateCount
            }
            function get_pid_state(uint64 _pid) pure private returns (State) {
                uint64 n = 8;
                for (uint16 i = 1; i < 10; ++i) {
                    if ((i % 3) == 0) {
                        n *= _pid / uint64(i);
                    } else {
                        n /= 3;
                    }
                }
        
                return State(n % uint64(State.StateCount));
            }
		}"#;

    assert_lir_str_eq(
        src,
        cfg_no,
        r#"private function sol#2 test::test::function::get_pid_state__uint64 (uint64) returns (uint8):
block#0 entry:
    uint64 %_pid = uint64(arg#0);
    uint64 %n = 8;
    uint16 %i = 1;
    br block#2;

block#1 body:
    uint16 %temp.ssa_ir.6 = uint16(%i) (u)% uint16(3);
    bool %temp.ssa_ir.5 = uint16(%temp.ssa_ir.6) == uint16(0);
    cbr bool(%temp.ssa_ir.5) block#5 else block#6;

block#2 cond:
    bool %temp.ssa_ir.7 = uint16(%i) (u)< uint16(10);
    cbr bool(%temp.ssa_ir.7) block#1 else block#4;

block#3 next:
    uint16 %temp.4 = uint16(%i) + uint16(1);
    uint16 %i = uint16(%temp.4);
    br block#2;

block#4 endfor:
    uint64 %temp.ssa_ir.9 = uint64(%n) (u)% uint64(5);
    uint8 %temp.ssa_ir.8 = (trunc uint64(%temp.ssa_ir.9) to uint8);
    return uint8(%temp.ssa_ir.8);

block#5 then:
    uint64 %temp.ssa_ir.11 = (zext uint16(%i) to uint64);
    uint64 %temp.ssa_ir.10 = uint64(%_pid) (u)/ uint64(%temp.ssa_ir.11);
    uint64 %n = uint64(%n) * uint64(%temp.ssa_ir.10);
    br block#7;

block#6 else:
    uint64 %n = uint64(%n) (u)/ uint64(3);
    br block#7;

block#7 endif:
    br block#3;"#,
    )
}

#[test]
fn test_test_more_() {
    let cfg_no = 0;

    // read the example.sol file
    let src = r#"
		contract test {
            enum suit { club, diamonds, hearts, spades }
            enum value { two, three, four, five, six, seven, eight, nine, ten, jack, queen, king, ace }
            struct card {
                value v;
                suit s;
            }
			function score_card(card memory c) public pure returns (uint32 score) {
                if (c.s == suit.hearts) {
                    if (c.v == value.ace) {
                        score = 14;
                    }
                    if (c.v == value.king) {
                        score = 13;
                    }
                    if (c.v == value.queen) {
                        score = 12;
                    }
                    if (c.v == value.jack) {
                        score = 11;
                    }
                }
                // all others score 0
            }
		}"#;

    assert_lir_str_eq(
        src,
        cfg_no,
        r#"public function sol#2 test::test::function::score_card__test.card (ptr<struct.0>) returns (uint32):
block#0 entry:
    ptr<struct.0> %c = ptr<struct.0>(arg#0);
    uint32 %score = 0;
    ptr<uint8> %temp.ssa_ir.4 = access ptr<struct.0>(%c) member 1;
    uint8 %temp.ssa_ir.3 = *ptr<uint8>(%temp.ssa_ir.4);
    bool %temp.ssa_ir.2 = uint8(%temp.ssa_ir.3) == uint8(2);
    cbr bool(%temp.ssa_ir.2) block#1 else block#2;

block#1 then:
    ptr<uint8> %temp.ssa_ir.7 = access ptr<struct.0>(%c) member 0;
    uint8 %temp.ssa_ir.6 = *ptr<uint8>(%temp.ssa_ir.7);
    bool %temp.ssa_ir.5 = uint8(%temp.ssa_ir.6) == uint8(12);
    cbr bool(%temp.ssa_ir.5) block#3 else block#4;

block#2 endif:
    return uint32(%score);

block#3 then:
    uint32 %score = 14;
    br block#4;

block#4 endif:
    ptr<uint8> %temp.ssa_ir.10 = access ptr<struct.0>(%c) member 0;
    uint8 %temp.ssa_ir.9 = *ptr<uint8>(%temp.ssa_ir.10);
    bool %temp.ssa_ir.8 = uint8(%temp.ssa_ir.9) == uint8(11);
    cbr bool(%temp.ssa_ir.8) block#5 else block#6;

block#5 then:
    uint32 %score = 13;
    br block#6;

block#6 endif:
    ptr<uint8> %temp.ssa_ir.13 = access ptr<struct.0>(%c) member 0;
    uint8 %temp.ssa_ir.12 = *ptr<uint8>(%temp.ssa_ir.13);
    bool %temp.ssa_ir.11 = uint8(%temp.ssa_ir.12) == uint8(10);
    cbr bool(%temp.ssa_ir.11) block#7 else block#8;

block#7 then:
    uint32 %score = 12;
    br block#8;

block#8 endif:
    ptr<uint8> %temp.ssa_ir.16 = access ptr<struct.0>(%c) member 0;
    uint8 %temp.ssa_ir.15 = *ptr<uint8>(%temp.ssa_ir.16);
    bool %temp.ssa_ir.14 = uint8(%temp.ssa_ir.15) == uint8(9);
    cbr bool(%temp.ssa_ir.14) block#9 else block#10;

block#9 then:
    uint32 %score = 11;
    br block#10;

block#10 endif:
    br block#2;"#,
    )
}

#[test]
fn test_init_struct() {
    let cfg_no = 0;

    // read the example.sol file
    let src = r#"
		contract test {
            enum suit { club, diamonds, hearts, spades }
            enum value { two, three, four, five, six, seven, eight, nine, ten, jack, queen, king, ace }
            struct card {
                value v;
                suit s;
            }
			function ace_of_spaces() public pure returns (card memory) {
                return card({s: suit.spades, v: value.ace });
            }
		}"#;

    assert_lir_str_eq(
        src,
        cfg_no,
        r#"public function sol#2 test::test::function::ace_of_spaces () returns (ptr<struct.0>):
block#0 entry:
    ptr<struct.0> %temp.ssa_ir.1 = struct { uint8(12), uint8(3) };
    return ptr<struct.0>(%temp.ssa_ir.1);"#,
    )
}
