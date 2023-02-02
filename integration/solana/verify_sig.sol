
contract verify_sig {
	function verify(address addr, bytes message, bytes signature) public view returns (bool) {
		return signatureVerify(addr, message, signature);
	}
}
