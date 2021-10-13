contract balances {
	function get_balance(address addr) public view returns (uint64) {
		return addr.balance;
	}
}
