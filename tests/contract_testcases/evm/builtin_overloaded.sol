contract C {
	bytes constant bs = "abcdefgh";
	int8 constant v = bs.readInt8(0);

	function test1() public {
		bs.readInt8("foo");
	}

	function test2() public {
		bs.readInt8(1, "foo");
	}
}

// ---- Expect: diagnostics ----
// error: 3:20-34: cannot call function in constant expression
// error: 6:15-20: implicit conversion to uint32 from bytes3 not allowed
// error: 10:6-14: builtin function 'readInt8' expects 1 arguments, 2 provided
