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

// ---- Expect: diagnostics ----
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
