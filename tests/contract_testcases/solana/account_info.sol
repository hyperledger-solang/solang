contract c {
	AccountInfo ai;

	function pub(AccountInfo) public returns (AccountInfo) {}
	function notpub(AccountInfo) private returns (AccountInfo) {
		AccountInfo ai = tx.accounts[1];
		ai.key = msg.sender;
		ai.lamports += 1;
		return tx.accounts[1];
	}
}
