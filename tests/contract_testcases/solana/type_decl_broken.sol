type GlobalFoo is address payable;

contract c {
	struct foo { int f1; }
	type foo is int[2];
	type foo is int;
	struct GlobalFoo { int f1; }

	type Value is uint128;
	struct Value { int f1; }

	function inc_and_wrap(int128 v) public returns (Value) {
		return Value.wrap(v + 1);
	}

	function dec_and_unwrap(Value v) public returns (uint128) {
		return Value.unwrap(v) - 1;
	}
}

// ----
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
