import "type_decl_broken.sol";

struct S {
	in f1;
}

function f(S s) returns (int) {
	return s.f1;
}

function g(S s) {
	s.f1 = "bla";
	s = S({f1: 2});
	s = S("feh");
	s = 1;
}
