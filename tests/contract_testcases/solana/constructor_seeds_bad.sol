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

// ---- Expect: diagnostics ----
// error: 4:8-9: 'x' not found
// error: 5:8-11: 'foo' not found
// error: 11:8-11: duplicate @bump annotation for constructor
// 	note 10:2-12: previous @bump
// error: 16:8-11: conversion from string to bytes not possible
