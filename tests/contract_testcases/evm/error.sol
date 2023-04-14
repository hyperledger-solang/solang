
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
