contract c1 {
	// argument foo doesn't exist
	// seed not found
	@seed(x)
	@bump(foo)
	constructor() {}

	// only one bump allowed
	@seed("Fickle")
	@bump(foo)
	@bump(foo)
	constructor(bytes1 foo) {}

	@seed("feh")
	@seed(foo)
	@seed(bar)
	@seed(baz)
	@bump(b)
	constructor(bytes foo, string bar, bytes baz, uint8 b) {}
}
