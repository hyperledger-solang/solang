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

// ----
// error (58-116): unknown annotation 'program_id' on contract c1
// error (133-145): unknown annotation seed for constructor
// error (147-155): unknown annotation bump for constructor
// error (157-167): unknown annotation seed for constructor
// error (169-184): unknown annotation space for constructor
// error (246-258): unknown annotation seed for function
// error (260-268): unknown annotation bump for function
// error (270-330): unknown annotation payer for function
// error (332-341): unknown annotation space for function
