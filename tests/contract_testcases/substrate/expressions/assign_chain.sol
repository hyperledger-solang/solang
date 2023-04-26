
struct S { bool f1; }

contract c {
	d d1;
	S s1;

	function f() public returns (int64) {
		d l = d1 = new d();
		return l.v();
	}

	function g() public returns (bool) {
		S s = S(false);
		bool l = s.f1 = true;

		return l;
	}
	function h() public returns (bool) {
		S storage s = s1;
		bool l = s.f1 = true;

		return l;
	}
}

contract d {
	int64 public v;
}

// ----
// warning (38-42): storage variable 'd1' has been assigned, but never read
// warning (134-168): function can be declared 'pure'
// warning (175-176): local variable 's' has been assigned, but never read
