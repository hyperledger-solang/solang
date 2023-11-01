contract balances {
    @account(acc1)
	function get_balance() external view returns (uint64) {
		return tx.accounts.acc1.lamports;
	}

    @mutableAccount(acc1)
    @mutableAccount(acc2)
	function transfer(uint64 amount) external {
		tx.accounts.acc1.lamports -= amount;
        tx.accounts.acc2.lamports += amount;
	}

}
