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
