
contract builtins {
	function hash_ripemd160(bytes memory bs) public pure returns (bytes20) {
		return ripemd160(bs);
	}

	function hash_kecccak256(bytes memory bs) public pure returns (bytes32) {
		return keccak256(bs);
	}

	function hash_sha256(bytes memory bs) public pure returns (bytes32) {
		return sha256(bs);
	}

	function mr_now() public view returns (uint64) {
		return block.timestamp;
	}
}
