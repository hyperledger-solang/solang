contract builtins2 {
    function hash_blake2_128(bytes memory bs) public pure returns (bytes16) {
		return blake2_128(bs);
	}

    function hash_blake2_256(bytes memory bs) public pure returns (bytes32) {
		return blake2_256(bs);
	}

    function block_height() public view returns (uint64) {
        return block.number;
    }

    function burn_gas(uint64 x) public view returns (uint64) {
        // just burn some gas
        for (uint64 i = 0; i < x; i++) {
            uint64 no = block.number;
        }

        return gasleft();
    }
}