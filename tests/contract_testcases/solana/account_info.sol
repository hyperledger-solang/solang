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

// ----
// error (32-43): variable cannot be of builtin type 'struct AccountInfo'
// error (63-74): parameter of type 'struct AccountInfo' not alowed in public or external functions
// error (92-103): return type 'struct AccountInfo' not allowed in public or external functions
// error (157-185): builtin struct 'AccountInfo' cannot be created using struct literal
// error (240-254): builtin struct 'AccountInfo' cannot be created using struct literal
// error (362-365): struct 'AccountInfo' field 'key' is readonly
