contract C {
	function f() public {
		// Three different parse tree for callargs with new
		(D.new{value: 1})();
		(D.new){value: 1}();
		D.new{value: 1}();
	}
	function g() external {
		// Three different parse tree for callargs
		D.func{value: 1}();
		(D.func){value: 1}();
		(D.func{value: 1})();
	}
}

@program_id("A2tWahcQqU7Mic5o4nGWPKt9rQaLVyh7cyF4MmCXksJt")
contract D {
	constructor() payable {}
	function func() payable external {}
}

// ---- Expect: diagnostics ----
// error: 4:3-22: accounts are required for calling a contract. You can either provide the accounts with the {accounts: ...} call argument or change this function's visibility to external
// error: 4:10-18: Solana Cross Program Invocation (CPI) cannot transfer native value. See https://solang.readthedocs.io/en/latest/language/functions.html#value_transfer
// error: 10:10-18: Solana Cross Program Invocation (CPI) cannot transfer native value. See https://solang.readthedocs.io/en/latest/language/functions.html#value_transfer
// error: 11:12-20: Solana Cross Program Invocation (CPI) cannot transfer native value. See https://solang.readthedocs.io/en/latest/language/functions.html#value_transfer
// error: 12:11-19: Solana Cross Program Invocation (CPI) cannot transfer native value. See https://solang.readthedocs.io/en/latest/language/functions.html#value_transfer
