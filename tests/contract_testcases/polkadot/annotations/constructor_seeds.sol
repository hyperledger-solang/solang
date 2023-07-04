contract c {
	@seed(foo)
	@bump(bar)
	constructor(bytes foo, uint8 bump) {}
}

// ---- Expect: diagnostics ----
// error: 1:1-5:2: contracts without public storage or functions are not allowed on Polkadot. Consider declaring this contract abstract: 'abstract contract c'
// error: 2:2-12: unknown annotation seed for constructor
// error: 3:2-12: unknown annotation bump for constructor
