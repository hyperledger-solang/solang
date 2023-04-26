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

// ----
// error (13-13): address literal @#$! invalid character '@'
// error (139-142): conversion from string to bytes not possible
// error (145-153): @bump should be after the last @seed
// 	note (155-165): location of @seed annotation
// error (180-181): 'a' not found
// error (259-259): address literal 102 invalid character '0'
// error (345-346): duplicate @bump annotation for constructor
// 	note (329-337): previous @bump
// error (356-407): address literal Chi1doxDSNjrmbZ5sq3H2cXyTq3KNfGepmbhyHaxcr incorrect length of 31
// error (441-450): duplicate @space annotation for constructor
// 	note (423-439): previous @space
// error (522-538): annotion takes an account, for example '@program_id("BBH7Xi5ddus5EoQhzJLgyodVxJJGkvBRCY5AhBA1jwUr")'
// error (624-625): duplicate @bump annotation for constructor
// 	note (608-616): previous @bump
// error (690-701): duplicate @payer annotation for constructor
// 	note (628-688): previous @payer
// error (721-730): duplicate @space annotation for constructor
// 	note (703-719): previous @space
// error (801-813): unknown annotation seed for function
// error (815-823): unknown annotation bump for function
// error (825-885): unknown annotation payer for function
// error (887-896): unknown annotation space for function
// error (974-987): @payer annotation required for constructor
