contract spl {
	function foo() public returns (bool, address) {
		AccountMeta meta = AccountMeta(address(msg.sender), true, false);
		return (meta.is_writable, meta.pubkey);
	}

	function bar(address x) public returns (bool, address) {
		AccountMeta[2] meta = [
			AccountMeta(x, true, true),
			AccountMeta({pubkey: x, is_writable: false, is_signer: false})
		];

		return (meta[1].is_writable, meta[0].pubkey);
	}

}
