// RUN: --emit cfg
contract c {
// BEGIN-CHECK: c::test1
	function test1() public pure {
		bytes x = "foo1";
		// x is not being used, so it can be a slice
// CHECK: alloc slice uint32 4 "foo1"
	}

// BEGIN-CHECK: c::test2
	function test2() public pure {
		bytes x = "foo2";

		x[1] = 0;
		// x is being modified, so it must be a vector
// CHECK: alloc bytes uint32 4 "foo2"
	}

	function foo(bytes x) pure internal {

	}

// BEGIN-CHECK: c::test3
	function test3() public pure {
		bytes x = "foo3";

		foo(x);
		// no slices for function arguments yet, so it must be a vector
// CHECK: alloc bytes uint32 4 "foo3"
	}


// BEGIN-CHECK: c::test4
	function test4() public pure {
		string x = "foo4";

		// a bunch of stuff that does not need a vector
		if (x == "bar") {
			bool y = true;
		}

		string y = x + "if";

		print(x);
// CHECK: alloc slice uint32 4 "foo4"
	}

// BEGIN-CHECK: c::test5
	function test5() public pure {
		bytes x = "foo5";

		x.push(0);
		// push modifies vectotr
// CHECK: alloc bytes uint32 4 "foo5"
	}

// BEGIN-CHECK: c::test6
	function test6() public pure {
		bytes x = "foo6";

		x.pop();
		// pop modifies vectotr
// CHECK: alloc bytes uint32 4 "foo6"
	}


// BEGIN-CHECK: c::test7
	function test7() public pure {
		bytes x = "foo7";

		bytes y = x;
		y[1] = 0;

		// x modified via y
// CHECK: alloc bytes uint32 4 "foo7"
	}

// BEGIN-CHECK: c::test8
	function test8() public pure {
		string x = "foo8";

		bytes y = bytes(x);
		y[1] = 0;

		// x modified via y
// CHECK: alloc string uint32 4 "foo8"
	}
}