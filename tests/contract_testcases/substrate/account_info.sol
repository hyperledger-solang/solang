abstract contract c {
	AccountInfo ai;

	function pub(AccountInfo) public returns (AccountInfo) {}
	function notpub(AccountInfo) private returns (AccountInfo) {}
}
