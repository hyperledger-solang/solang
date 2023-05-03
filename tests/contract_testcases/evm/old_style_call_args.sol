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

// ---- Expect: diagnostics ----
// error: 3:17-25: deprecated call argument syntax '.value(...)' is not supported, use '{value: ...}' instead
// error: 3:26-32: deprecated call argument syntax '.gas(...)' is not supported, use '{gas: ...}' instead
// error: 3:33-39: deprecated call argument syntax '.foo(...)' is not supported, use '{foo: ...}' instead
// error: 7:13-14: type with arguments expected
// error: 11:9-14: missing constructor arguments to d
