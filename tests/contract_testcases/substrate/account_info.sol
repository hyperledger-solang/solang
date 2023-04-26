contract c {
	AccountInfo ai;

	function pub(AccountInfo) public returns (AccountInfo) {}
	function notpub(AccountInfo) private returns (AccountInfo) {}
}

// ----
// error (14-25): type 'AccountInfo' not found
// error (45-56): type 'AccountInfo' not found
// error (74-85): type 'AccountInfo' not found
// error (107-118): type 'AccountInfo' not found
// error (137-148): type 'AccountInfo' not found
