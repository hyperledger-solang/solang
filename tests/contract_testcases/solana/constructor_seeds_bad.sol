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

// ----
// error (71-72): 'x' not found
// error (81-84): 'foo' not found
// error (167-170): duplicate @bump annotation for constructor
// 	note (149-159): previous @bump
// error (234-237): conversion from string to bytes not possible
