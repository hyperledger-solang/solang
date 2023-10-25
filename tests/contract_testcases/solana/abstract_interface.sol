abstract contract A {
	function v(int) public virtual;
}
contract C {
	function t(address id) public {
		A.v{program_id: id}(1);
	}
}

// ---- Expect: diagnostics ----
// error: 6:3-25: accounts are required for calling a contract. You can either provide the accounts with the {accounts: ...} call argument or change this function's visibility to external
