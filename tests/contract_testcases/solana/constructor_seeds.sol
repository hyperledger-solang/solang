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

// ---- Expect: diagnostics ----
// error: 1:14: address literal @#$! invalid character '@'
// error: 8:8-11: conversion from string to bytes not possible
// error: 9:2-10: @bump should be after the last @seed
// 	note 10:2-12: location of @seed annotation
// error: 11:15-16: 'a' not found
// error: 15:15: address literal 102 invalid character '0'
// error: 20:8-9: duplicate @bump annotation for constructor
// 	note 19:2-10: previous @bump
// error: 21:9-60: address literal Chi1doxDSNjrmbZ5sq3H2cXyTq3KNfGepmbhyHaxcr incorrect length of 31
// error: 24:2-11: duplicate @space annotation for constructor
// 	note 23:2-18: previous @space
// error: 28:1-17: annotion takes an account, for example '@program_id("BBH7Xi5ddus5EoQhzJLgyodVxJJGkvBRCY5AhBA1jwUr")'
// error: 33:8-9: duplicate @bump annotation for constructor
// 	note 32:2-10: previous @bump
// error: 35:2-13: duplicate @payer annotation for constructor
// 	note 34:2-62: previous @payer
// error: 37:2-11: duplicate @space annotation for constructor
// 	note 36:2-18: previous @space
// error: 40:2-14: unknown annotation seed for function
// error: 41:2-10: unknown annotation bump for function
// error: 42:2-62: unknown annotation payer for function
// error: 43:2-11: unknown annotation space for function
// error: 49:8-21: @payer annotation required for constructor
