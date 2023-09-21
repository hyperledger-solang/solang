@program_id("@#$!")
contract c1 {
	/**
	 * Multiple seeds allowed. Space has an incorrect expression
	 */
	@seed("feh")
	@space(102 + a)
	constructor(@seed bytes foo, @seed string bar, @seed bytes baz, @bump uint8 b) {}
}

@program_id("102")
contract c2 {
	/// Only one bump allowed.
	@seed(hex"41420044")
	@bump(5)
	@payer(address"Chi1doxDSNjrmbZ5sq3H2cXyTq3KNfGepmbhyHaxcr")
	@payer(bar)
	@space(1025 + 5)
	@space(4)
	constructor(bytes foo, bytes baz, @bump uint8 b) {}
}

@program_id(foo)
contract c3 {
	/**  Only one bump and one space allowed. */
	@seed(hex"41420044")
	@payer(my_account)
	@payer(bar)
	@space(1025 + 5)
	@space(4)
	constructor(bytes foo, address payable bar, bytes baz, @bump bytes1 b, @bump bytes1 c) {}

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

// ---- Expect: diagnostics ----
// error: 1:14: address literal @#$! invalid character '@'
// error: 7:15-16: 'a' not found
// error: 11:15: address literal 102 invalid character '0'
// error: 16:2-61: invalid parameter for annotation
// error: 19:9-10: duplicate @space annotation for constructor
// 	note 18:2-18: previous @space
// error: 20:36-41: duplicate @bump annotation for constructor
// 	note 15:2-10: previous @bump
// error: 23:1-17: annotion takes an account, for example '@program_id("BBH7Xi5ddus5EoQhzJLgyodVxJJGkvBRCY5AhBA1jwUr")'
// error: 28:2-13: duplicate @payer annotation for constructor
// 	note 27:2-20: previous @payer
// error: 30:9-10: duplicate @space annotation for constructor
// 	note 29:2-18: previous @space
// error: 31:73-78: duplicate @bump annotation for constructor
// 	note 31:57-62: previous @bump
// error: 33:2-14: unknown annotation seed for function
// error: 34:2-10: unknown annotation bump for function
// error: 35:2-62: unknown annotation payer for function
// error: 36:2-11: unknown annotation space for function
// error: 45:2-16: @payer annotation required for constructor
