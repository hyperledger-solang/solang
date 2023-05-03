
error E1(int bar, bool);
error E2(int bar, bool foo);

contract e {
	error E3(int bar, bool foo);
}

contract c is e {
	error E4(int bar, bool foo);

	function named1() public {
		if (1 != 0) {
			revert E1({bar: 1, foo: true});
		} else if (1 != 0) {
			revert E2({bar: 1});
		} else if (1 != 0) {
			revert E3({bar: 1, foo: true, baz: "hey"});
		} else if (1 != 0) {
			revert e.E3({bar: 1, foo: true, foo: false});
		} else {
			revert E4({bar: 2, baz: true});
		}
	}

	function pos1() public {
		if (1 != 0) {
			revert E1("feh", false);
		} else if (1 != 0) {
			revert E2(1);
		} else if (1 != 0) {
			revert e.E2(1, true, "hey");
		} else {
			revert E3(2, true);
		}

	}
}

// ---- Expect: diagnostics ----
// error: 14:11-13: error 'E1' has 1 unnamed fields
// 	note 2:7-9: definition of 'E1'
// error: 14:23-26: error 'E1' has no field called 'foo'
// 	note 2:7-9: definition of 'E1'
// error: 16:11-13: missing field 'foo'
// 	note 3:7-9: definition of 'E2'
// error: 18:34-37: error 'E3' has no field called 'baz'
// 	note 6:8-10: definition of 'E3'
// error: 20:36-39: duplicate argument with name 'foo'
// error: 22:11-13: missing field 'foo'
// 	note 10:8-10: definition of 'E4'
// error: 22:23-26: error 'E4' has no field called 'baz'
// 	note 10:8-10: definition of 'E4'
// error: 28:14-19: implicit conversion to int256 from bytes3 not allowed
// error: 30:11-13: error 'E2' has 2 fields, 1 provided
// 	note 3:7-9: definition of 'E2'
// error: 32:11-15: error 'E2' has 2 fields, 3 provided
// 	note 3:7-9: definition of 'E2'
