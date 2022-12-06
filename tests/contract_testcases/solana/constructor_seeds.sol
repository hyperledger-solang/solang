@program_id("@#$!")
contract c1 {
	/**
	 * Multiple seeds allowed, but bump must be after last seed.
	 */
	@seed("feh")
	@seed(foo)
	@seed(bar)
	@bump(b)
	@seed(baz)
	@space(102 + a)
	constructor(bytes foo, string bar, bytes baz, uint8 b) {}
}

@program_id("102")
contract c2 {
	/// Only one bump allowed.
	@seed(hex"41420044")
	@bump(b)
	@bump(5)
	@payer(address"Chi1doxDSNjrmbZ5sq3H2cXyTq3KNfGepmbhyHaxcr")
	@payer(bar)
	@space(1025 + 5)
	@space(4)
	constructor(bytes foo, address payable bar, bytes baz, uint8 b) {}
}

@program_id(foo)
contract c3 {
	/**  Only one bump allowed. */
	@seed(hex"41420044")
	@bump(b)
	@bump(5)
	@payer(address"Chi1doxDSNjrmbZ5sq3H2cXyTq3KNfGepmbhyHaxcr8")
	@payer(bar)
	@space(1025 + 5)
	@space(4)
	constructor(bytes foo, address payable bar, bytes baz, uint8 b) {}

	@seed("meh")
	@bump(1)
	@payer(address"Chi1doxDSNjrmbZ5sq3H2cXyTq3KNfGepmbhyHaxcr8")
	@space(5)
	function func() public {}
}

contract c4 {
	/** Payer is required */
	@seed(hex"41420044")
	@space(4)
	@bump(1)
	constructor() {}
}
