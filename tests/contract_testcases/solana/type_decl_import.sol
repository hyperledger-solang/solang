import "type_decl.sol" as IMP;

contract d {
	function f(IMP.x c) public {
		IMP.Addr a = IMP.Addr.wrap(payable(this));
		IMP.x.Binary b = IMP.x.Binary.wrap(false);

		c.f(a, b);
	}
}

// ----
// warning (69-100): function can be declared 'pure'
