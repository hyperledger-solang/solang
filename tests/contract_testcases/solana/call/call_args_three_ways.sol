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

// ----
// error (98-117): 'address' call argument required on solana
// error (105-113): Solana Cross Program Invocation (CPI) cannot transfer native value. See https://solang.readthedocs.io/en/latest/language/functions.html#value_transfer
// error (261-269): Solana Cross Program Invocation (CPI) cannot transfer native value. See https://solang.readthedocs.io/en/latest/language/functions.html#value_transfer
// error (285-293): Solana Cross Program Invocation (CPI) cannot transfer native value. See https://solang.readthedocs.io/en/latest/language/functions.html#value_transfer
// error (308-316): Solana Cross Program Invocation (CPI) cannot transfer native value. See https://solang.readthedocs.io/en/latest/language/functions.html#value_transfer
