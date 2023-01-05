import 'solana';

contract builtins {
	function hash_ripemd160(bytes bs) public pure returns (bytes20) {
		return ripemd160(bs);
	}

	function hash_kecccak256(bytes bs) public pure returns (bytes32) {
		return keccak256(bs);
	}

	function hash_sha256(bytes bs) public pure returns (bytes32) {
		return sha256(bs);
	}

	function mr_now() public view returns (uint64) {
		return block.timestamp;
	}

	function mr_slot() public view returns (uint64) {
		return block.slot;
	}

	function pda(bytes seed1, bytes seed2, address addr) public pure returns (address) {
		return create_program_address([seed1, seed2], addr);
	}

	function pda_with_bump(bytes seed1, bytes seed2, address addr) public pure returns (address, bytes1) {
		return try_find_program_address([seed1, seed2], addr);
	}
}
