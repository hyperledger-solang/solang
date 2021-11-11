contract balances {
	function get_balance(address addr) public view returns (uint64) {
		return addr.balance;
	}

	function transfer(address payable addr, uint64 amount) public {
		addr.transfer(amount);
	}

	function send(address payable addr, uint64 amount) public returns (bool) {
		return addr.send(amount);
	}

	function pay_me() public payable {
		uint64 v = msg.value;

		print("Thank you very much for {}".format(v));
	}
}
