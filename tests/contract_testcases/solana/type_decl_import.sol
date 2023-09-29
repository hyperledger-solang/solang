import "./type_decl.sol" as IMP;

contract d {
	function f(address pid) public {
		IMP.Addr a = IMP.Addr.wrap(payable(this));
		IMP.x.Binary b = IMP.x.Binary.wrap(false);

		IMP.x.f{program_id: pid}(a, b);
	}
}

// ---- Expect: diagnostics ----
// warning: 7:2-33: function can be declared 'pure'
