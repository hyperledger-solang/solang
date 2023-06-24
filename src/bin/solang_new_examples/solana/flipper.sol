@program_id("F1ipperKF9EfD821ZbbYjS319LXYiBmjhzkkf5a26rC")
contract flipper {
	bool private value;

	/// Constructor that initializes the `bool` value to the given `init_value`.
	@payer(payer)
	constructor(bool initvalue) {
		value = initvalue;
	}

	/// A message that can be called on instantiated contracts.
	/// This one flips the value of the stored `bool` from `true`
	/// to `false` and vice versa.
	function flip() public {
		value = !value;
	}

	/// Simply returns the current value of our `bool`.
	function get() public view returns (bool) {
		return value;
	}
}
