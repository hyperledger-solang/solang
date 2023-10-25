abstract contract b {
	receive() external payable virtual;
	fallback() external virtual;
}

contract c is b {
}
// ---- Expect: diagnostics ----
// error: 6:1-7:2: contract 'c' missing override for fallback function
// 	note 3:2-29: declaration of fallback function
// error: 6:1-7:2: contract 'c' missing override for receive function
// 	note 2:2-36: declaration of receive function
