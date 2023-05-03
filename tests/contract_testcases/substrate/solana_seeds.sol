/// Make sure that Solana annotations do not do anything

@program_id("Chi1doxDSNjrmbZ5sq3H2cXyTq3KNfGepmbhyHaxcre")
contract c1 {

	@seed("feh")
	@bump(b)
	@seed(baz)
	@space(102 + a)
	constructor(bytes foo, string bar, bytes baz, uint8 b) {}

	@seed("meh")
	@bump(1)
	@payer(address"Chi1doxDSNjrmbZ5sq3H2cXyTq3KNfGepmbhyHaxcr8")
	@space(5)
	function func() public {}
}

// ---- Expect: diagnostics ----
// error: 3:1-59: unknown annotation 'program_id' on contract c1
// error: 6:2-14: unknown annotation seed for constructor
// error: 7:2-10: unknown annotation bump for constructor
// error: 8:2-12: unknown annotation seed for constructor
// error: 9:2-17: unknown annotation space for constructor
// error: 12:2-14: unknown annotation seed for function
// error: 13:2-10: unknown annotation bump for function
// error: 14:2-62: unknown annotation payer for function
// error: 15:2-11: unknown annotation space for function
