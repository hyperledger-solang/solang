contract c1 {
	// argument foo doesn't exist
	// seed not found
	@seed(x)
	@bump(foo)
	constructor() {}

	// only one space allowed
	@seed("Fickle")
	@space(3)
	constructor(@space uint64 arg1) {}

	// only one bump allowed
	@seed("Tree")
	@bump(90)
	constructor(@bump bytes1 b1) {}

	@seed("feh")
	constructor(@seed bytes foo, @seed string bar, @seed bytes baz, @bump uint64 b) {}
}

// ---- Expect: diagnostics ----
// error: 4:8-9: 'x' not found
// error: 5:8-11: 'foo' not found
// error: 11:14-20: duplicate @space annotation for constructor
// 	note 10:2-11: previous @space
// error: 16:14-19: duplicate @bump annotation for constructor
// 	note 15:2-11: previous @bump
// error: 19:66-71: implicit conversion to bytes1 from uint64 not allowed
