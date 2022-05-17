contract Sign {
    /**
     * @dev Recover signer address from a message by using their signature
     * @param hash bytes32 message, the hash is the signed message. What is recovered is the signer address.
     * @param signature bytes signature, the signature is generated using web3.eth.sign()
     */
    function recover(bytes32 hash, bytes memory signature) public pure returns (address) {
        // Divide the signature in r, s and v variables
        bytes32 r;
        bytes32 s;
        uint32 v;

        bytes memory rA = new bytes(32);
        bytes memory sA = new bytes(32);

        for (uint256 i = 0; i < 32; i += 1) {
            rA[i] = signature[i];
            sA[i] = signature[i + 32];
        }

        r = bytes32(rA);
        s = bytes32(sA);
        
        if (signature.length == 65) {
            v = uint8(signature[65]);
        } else {
            v = uint8(signature[64]) * 256 + uint8(signature[65]);
        }

        // If the signature is valid (and not malleable), return the signer address
        return ecrecover(hash, v, r, s);
    }

    /**
     * toEthSignedMessageHash
     * @dev prefix a bytes32 value with "\x19Ethereum Signed Message:"
     * and hash the result
     */
    function toEthSignedMessageHash(bytes32 hash) public pure returns (bytes32) {
        // 32 is the length in bytes of hash,
        // enforced by the type signature above
        return keccak256(abi.encodePacked("\x19Ethereum Signed Message:\n32", hash));
    }
}
