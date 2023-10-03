import "./type_decl.sol" as IMP;

contract d {
	function f(address pid) public {
		IMP.Addr a = IMP.Addr.wrap(payable(this));
		IMP.x.Binary b = IMP.x.Binary.wrap(false);

		IMP.x.f{program_id: pid}(a, b);
	}
}

// ---- Expect: diagnostics ----
// error: 8:3-8: accounts are required for calling a contract. You can either provide the accounts with the {accounts: ...} call argument or change this function's visibility to external
// warning: 7:2-33: function can be declared 'pure'
