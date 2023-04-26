contract c {
	@seed(foo)
	@bump(bar)
	constructor(bytes foo, uint8 bump) {}
}

// ----
// error (0-77): contracts without public storage or functions are not allowed on Substrate. Consider declaring this contract abstract: 'abstract contract c'
// error (14-24): unknown annotation seed for constructor
// error (26-36): unknown annotation bump for constructor
