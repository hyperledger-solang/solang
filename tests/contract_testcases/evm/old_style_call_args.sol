contract c {
	function foo() public {
		d x = (new d).value(1).gas(2).foo(3)();
	}

	function bar() public {
		d x = new 1;
	}

	function baz() public {
		d x = new d;
	}
}

contract d {}

// ----
// error (54-62): deprecated call argument syntax '.value(...)' is not supported, use '{value: ...}' instead
// error (63-69): deprecated call argument syntax '.gas(...)' is not supported, use '{gas: ...}' instead
// error (70-76): deprecated call argument syntax '.foo(...)' is not supported, use '{foo: ...}' instead
// error (121-122): type with arguments expected
// error (161-166): missing constructor arguments to d
