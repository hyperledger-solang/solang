contract C {
	function f() public {
		// Three different parse tree for callargs with new
		D d = (new D{value: 1})();
		D dd = (new D){value: 1}();
		D ddd = new D{value: 1}();
	}
	function g(D d) public {
		// Three different parse tree for callargs
		d.func{value: 1}();
		(d.func){value: 1}();
		(d.func{value: 1})();
	}
}

@program_id("A2tWahcQqU7Mic5o4nGWPKt9rQaLVyh7cyF4MmCXksJt")
contract D {
	constructor() payable {}
	function func() payable public {}
}
