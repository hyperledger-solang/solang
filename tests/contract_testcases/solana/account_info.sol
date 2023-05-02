import "solana";

contract c {
	AccountInfo ai;

	function pub(AccountInfo) public returns (AccountInfo) {}

	function f() public {
		AccountInfo ai;
		ai = AccountInfo({lamports: "x"});
	}

	function f2() public {
		AccountInfo ai;
		ai = AccountInfo(1);
	}

	function notpub(AccountInfo) private returns (AccountInfo) {
		AccountInfo ai = tx.accounts[1];
		ai.key = address(this);
		ai.lamports += 1;
		return tx.accounts[1];
	}
}

// ---- Expect: diagnostics ----
// error: 4:2-13: variable cannot be of builtin type 'struct AccountInfo'
// error: 6:15-26: parameter of type 'struct AccountInfo' not alowed in public or external functions
// error: 6:44-55: return type 'struct AccountInfo' not allowed in public or external functions
// error: 10:8-36: builtin struct 'AccountInfo' cannot be created using struct literal
// error: 15:8-22: builtin struct 'AccountInfo' cannot be created using struct literal
// error: 20:6-9: struct 'AccountInfo' field 'key' is readonly
