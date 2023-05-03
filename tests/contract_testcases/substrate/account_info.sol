contract c {
	AccountInfo ai;

	function pub(AccountInfo) public returns (AccountInfo) {}
	function notpub(AccountInfo) private returns (AccountInfo) {}
}

// ---- Expect: diagnostics ----
// error: 2:2-13: type 'AccountInfo' not found
// error: 4:15-26: type 'AccountInfo' not found
// error: 4:44-55: type 'AccountInfo' not found
// error: 5:18-29: type 'AccountInfo' not found
// error: 5:48-59: type 'AccountInfo' not found
