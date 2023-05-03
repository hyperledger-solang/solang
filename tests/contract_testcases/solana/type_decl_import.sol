import "type_decl.sol" as IMP;

contract d {
	function f(IMP.x c) public {
		IMP.Addr a = IMP.Addr.wrap(payable(this));
		IMP.x.Binary b = IMP.x.Binary.wrap(false);

		c.f(a, b);
	}
}

// ---- Expect: diagnostics ----
// warning: 7:2-33: function can be declared 'pure'
