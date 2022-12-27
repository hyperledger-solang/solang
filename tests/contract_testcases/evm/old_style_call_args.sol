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
