import "solana";

contract c {
	AccountInfo ai;

	function pub(AccountInfo) public returns (AccountInfo) {}

	function f() public {
		AccountInfo ai;
		ai = AccountInfo({lamports: "x"});
	}

	function f2() public {
		AccountInfo ai;
		ai = AccountInfo(1);
	}

	function notpub(AccountInfo) private returns (AccountInfo) {
		AccountInfo ai = tx.accounts[1];
		ai.key = address(this);
		ai.lamports += 1;
		return tx.accounts[1];
	}
}
