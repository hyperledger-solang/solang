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
