contract hello_world {
	constructor() {
		print("Hello from the constructor");
	}

	function test() public pure {
		print("Hello from the test function");
	}
}
