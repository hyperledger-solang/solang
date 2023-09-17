contract signed {
	function recoverSignerFromSignature(uint8 v, bytes32 r, bytes32 s, bytes32 hash) pure external {
		address signer = ecrecover(hash, v, r, s);
		require(signer != address(0), "ECDSA: invalid signature");
	}
}

// ---- Expect: diagnostics ----
