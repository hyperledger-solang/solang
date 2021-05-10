
contract hash_functions {
	function hash_ripemd160(bytes bs) public returns (bytes20) {
		return ripemd160(bs);
	}

/* this relies on Solana providing the sol_keccak256() syscall, which is
 * not available yet
	function hash_kecccak256(bytes bs) public returns (bytes32) {
		return keccak256(bs);
	}
*/

	function hash_sha256(bytes bs) public returns (bytes32) {
		return sha256(bs);
	}

	function mr_now() public returns (uint64) {
		return block.timestamp;
	}
}
