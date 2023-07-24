import "./type_decl_broken.sol";

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

// ---- Expect: diagnostics ----
// error: 4:2-4: type 'in' not found
// error: 15:6-7: expected 'struct S', found integer
// error: 5:7-10: foo is already defined as a struct
// 	note 4:9-12: location of previous definition
// error: 5:14-20: 'int256[2]' is not an elementary value type
// error: 6:7-10: foo is already defined as a struct
// 	note 4:9-12: location of previous definition
// warning: 7:9-18: GlobalFoo is already defined as an user type
// 	note 1:6-15: location of previous definition
// error: 10:9-14: Value is already defined as an user type
// 	note 9:7-12: location of previous definition
// error: 13:21-26: implicit conversion would change sign from int136 to uint128
