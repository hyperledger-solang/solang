
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

// ----
// error (205-207): error 'E1' has 1 unnamed fields
// 	note (7-9): definition of 'E1'
// error (217-220): error 'E1' has no field called 'foo'
// 	note (7-9): definition of 'E1'
// error (263-265): missing field 'foo'
// 	note (32-34): definition of 'E2'
// error (333-336): error 'E3' has no field called 'baz'
// 	note (76-78): definition of 'E3'
// error (405-408): duplicate argument with name 'foo'
// error (440-442): missing field 'foo'
// 	note (127-129): definition of 'E4'
// error (452-455): error 'E4' has no field called 'baz'
// 	note (127-129): definition of 'E4'
// error (528-533): implicit conversion to int256 from bytes3 not allowed
// error (576-578): error 'E2' has 2 fields, 1 provided
// 	note (32-34): definition of 'E2'
// error (616-620): error 'E2' has 2 fields, 3 provided
// 	note (32-34): definition of 'E2'
