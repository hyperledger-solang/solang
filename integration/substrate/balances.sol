contract balances {
	function get_balance() public view returns (uint128) {
		return address(this).balance;
	}

	function transfer(address payable addr, uint128 amount) public {
		addr.transfer(amount);
	}

	function send(address payable addr, uint128 amount) public returns (bool) {
		return addr.send(amount);
	}

	function pay_me() public payable {
		uint128 v = msg.value;

		// Disabled for now, see #911
		// print("Thank you very much for {}".format(v));
	}
}
