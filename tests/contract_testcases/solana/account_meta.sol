import "solana" as sol;

contract spl {
	function foo() public returns (bool, address) {
		sol.AccountMeta meta = sol.AccountMeta(address(this), true, false);
		return (meta.is_writable, meta.pubkey);
	}

	function bar(address x) public returns (bool, address) {
		sol.AccountMeta[2] meta = [
			sol.AccountMeta(x, true, true),
			sol.AccountMeta({pubkey: x, is_writable: false, is_signer: false})
		];

		return (meta[1].is_writable, meta[0].pubkey);
	}

}

// ---- Expect: diagnostics ----
// warning: 4:2-47: function can be declared 'view'
// warning: 9:2-56: function can be declared 'pure'
