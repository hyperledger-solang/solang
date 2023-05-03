contract C {
	function f() public {
		// Three different parse tree for callargs with new
		D d = (new D{value: 1})();
		D dd = (new D){value: 1}();
		D ddd = new D{value: 1}();
	}
	function g(D d) public {
		// Three different parse tree for callargs
		d.func{value: 1}();
		(d.func){value: 1}();
		(d.func{value: 1})();
	}
}

@program_id("A2tWahcQqU7Mic5o4nGWPKt9rQaLVyh7cyF4MmCXksJt")
contract D {
	constructor() payable {}
	function func() payable public {}
}

// ---- Expect: diagnostics ----
// error: 4:9-28: 'address' call argument required on solana
// error: 4:16-24: Solana Cross Program Invocation (CPI) cannot transfer native value. See https://solang.readthedocs.io/en/latest/language/functions.html#value_transfer
// error: 10:10-18: Solana Cross Program Invocation (CPI) cannot transfer native value. See https://solang.readthedocs.io/en/latest/language/functions.html#value_transfer
// error: 11:12-20: Solana Cross Program Invocation (CPI) cannot transfer native value. See https://solang.readthedocs.io/en/latest/language/functions.html#value_transfer
// error: 12:11-19: Solana Cross Program Invocation (CPI) cannot transfer native value. See https://solang.readthedocs.io/en/latest/language/functions.html#value_transfer
