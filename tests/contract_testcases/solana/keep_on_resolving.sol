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

// ----
// error (44-46): type 'in' not found
// error (173-174): expected 'struct S', found integer
// error (79-82): foo is already defined as a struct
// 	note (57-60): location of previous definition
// error (86-92): 'int256[2]' is not an elementary value type
// error (100-103): foo is already defined as a struct
// 	note (57-60): location of previous definition
// warning (120-129): GlobalFoo is already defined as an user type
// 	note (5-14): location of previous definition
// error (175-180): Value is already defined as an user type
// 	note (149-154): location of previous definition
// error (272-277): implicit conversion would change sign from int136 to uint128
