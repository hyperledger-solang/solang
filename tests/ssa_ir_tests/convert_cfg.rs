use std::{ffi::OsStr, io::stdout};

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
    // let str = &cfg.to_string(&contract, &ns);
    // println!("=====================");
    // println!("cfg: {}", str);
    // println!("=====================");

    let converter = Converter::new(&ns, cfg);
    let new_cfg = converter.get_ssa_ir_cfg().unwrap();

    let printer = Printer {
        vartable: Box::new(new_cfg.vartable.clone()),
    };

    let expected_cfg_str = r#"public function sol#2 dynamicarray::dynamicarray::function::test ():
block#0 entry:
    uint32 %array_length.temp.1 = 3;
    ptr<int64[]> %a = alloc int64[][uint32(3)];
    uint32 %index.temp.3 = 0;
    bool %temp.ssa_ir.9 = uint32(0) (u)>= uint32(3);
    cbr bool(%temp.ssa_ir.9) block#1 else block#2;

block#1 out_of_bounds:
    assert_failure;

block#2 in_bounds:
    int64 %temp.2 = 1;
    ptr<int64[]> %temp.ssa_ir.11 = ptr<int64[]>(%a);
    uint32 %temp.ssa_ir.12 = 0;
    ptr<int64> %temp.ssa_ir.10 = ptr<int64[]>(%temp.ssa_ir.11)[uint32(%temp.ssa_ir.12)];
    store int64(1) to ptr<int64>(%temp.ssa_ir.10);
    uint32 %index.temp.5 = 1;
    bool %temp.ssa_ir.13 = uint32(1) (u)>= uint32(3);
    cbr bool(%temp.ssa_ir.13) block#3 else block#4;

block#3 out_of_bounds:
    assert_failure;

block#4 in_bounds:
    int64 %temp.4 = 2;
    ptr<int64[]> %temp.ssa_ir.15 = ptr<int64[]>(%a);
    uint32 %temp.ssa_ir.16 = 1;
    ptr<int64> %temp.ssa_ir.14 = ptr<int64[]>(%temp.ssa_ir.15)[uint32(%temp.ssa_ir.16)];
    store int64(2) to ptr<int64>(%temp.ssa_ir.14);
    uint32 %index.temp.7 = 2;
    bool %temp.ssa_ir.17 = uint32(2) (u)>= uint32(3);
    cbr bool(%temp.ssa_ir.17) block#5 else block#6;

block#5 out_of_bounds:
    assert_failure;

block#6 in_bounds:
    int64 %temp.6 = 3;
    ptr<int64[]> %temp.ssa_ir.19 = ptr<int64[]>(%a);
    uint32 %temp.ssa_ir.20 = 2;
    ptr<int64> %temp.ssa_ir.18 = ptr<int64[]>(%temp.ssa_ir.19)[uint32(%temp.ssa_ir.20)];
    store int64(3) to ptr<int64>(%temp.ssa_ir.18);
    int64 %temp.8 = push_mem ptr<int64[]>(%a) int64(4);
    uint32 %array_length.temp.1 = 4;
    bool %temp.ssa_ir.21 = uint32(4) == uint32(4);
    cbr bool(%temp.ssa_ir.21) block#7 else block#8;

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

#[test]
fn test_convert_cfg_1() {
    let src = r#"
// example.sol
contract example {
	// Process state
	enum State {
		Running,
		Sleeping,
		Waiting,
		Stopped,
		Zombie,
		StateCount
	}

	// Variables in contract storage
	State state;
	int32 pid;
	uint32 reaped = 3;

	// Constants
	State constant bad_state = State.Zombie;
	int32 constant first_pid = 1;

	// Our constructors
	constructor(int32 _pid) {
		// Set contract storage
		pid = _pid;
	}

	// Reading but not writing contract storage means function
	// can be declared view
	function is_zombie_reaper() public view returns (bool) {
		/* must be pid 1 and not zombie ourselves */
		return (pid == first_pid && state != State.Zombie);
	}

	// Returning a constant does not access storage at all, so
	// function can be declared pure
	function systemd_pid() public pure returns (uint32) {
		// Note that cast is required to change sign from
		// int32 to uint32
		return uint32(first_pid);
	}

	/// Convert celcius to fahrenheit
	function celcius2fahrenheit(int32 celcius) pure public returns (int32) {
		int32 fahrenheit = celcius * 9 / 5 + 32;

		return fahrenheit;
	}

	/// Convert fahrenheit to celcius
	function fahrenheit2celcius(int32 fahrenheit) pure public returns (int32) {
		return (fahrenheit - 32) * 5 / 9;
	}

	/// is this number a power-of-two
	function is_power_of_2(uint n) pure public returns (bool) {
		return n != 0 && (n & (n - 1)) == 0;
	}

	/// calculate the population count (number of set bits) using Brian Kerningham's way
	function population_count(uint n) pure public returns (uint count) {
		for (count = 0; n != 0; count++) {
			n &= (n - 1);
		}
	}

	/// calculate the power of base to exp
	function power(uint base, uint exp) pure public returns (uint) {
		return base ** exp;
	}

	/// returns true if the address is 0
	function is_address_zero(address a) pure public returns (bool) {
		return a == address(0);
	}

	/// reverse the bytes in an array of 8 (endian swap)
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

	/// This mocks a pid state
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

	/// Overloaded function with different return value!
	function get_pid_state() view private returns (uint32) {
		return reaped;
	}

	function reap_processes() public {
		uint32 n = 0;

		while (n < 100) {
			if (get_pid_state(n) == State.Zombie) {
				// reap!
				reaped += 1;
			}
			n++;
		}
	}

	function run_queue() public pure returns (uint16) {
		uint16 count = 0;
		// no initializer means its 0.
		uint32 n=0;

		do {
			if (get_pid_state(n) == State.Waiting) {
				count++;
			}
		}
		while (++n < 1000);

		return count;
	}

	// cards
	enum suit { club, diamonds, hearts, spades }
	enum value { two, three, four, five, six, seven, eight, nine, ten, jack, queen, king, ace }
	struct card {
		value v;
		suit s;
	}

	card card1 = card(value.two, suit.club);
	card card2 = card({s: suit.club, v: value.two});

	// This function does a lot of copying
	function set_card1(card memory c) public returns (card memory previous) {
		previous = card1;
		card1 = c;
	}

	/// return the ace of spades
	function ace_of_spaces() public pure returns (card memory) {
		return card({s: suit.spades, v: value.ace });
	}

	/// score card
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

    // iterate all cfgs
    for cfg in &contract.cfg {
        let str = &cfg.to_string(&contract, &ns);
        println!("=====================");
        println!("cfg: {}", str);
        println!("=====================");

        let converter = Converter::new(&ns, cfg);
        let new_cfg = converter.get_ssa_ir_cfg().unwrap();
        let printer = Printer {
            vartable: Box::new(new_cfg.vartable.clone()),
        };
        // use stdio to print the cfg
        printer.print_cfg(&mut stdout(), &new_cfg).unwrap();
    }
}
