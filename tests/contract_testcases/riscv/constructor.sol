// Tested by r55/tests/solang_constructor.rs.

/// `sum` shows whether the initializers ran before the constructor.
contract with_constructor {
	uint64 public initialized = 40;
	uint64 public from_constructor;
	uint64 public sum;

	constructor(uint64 arg) {
		from_constructor = arg;
		sum = initialized + arg;
	}
}

contract without_constructor {
	uint64 public initialized = 40;
	uint64 public not_initialized;
}
