contract selector {
	constructor() selector=hex"abcd" {}
	modifier m() selector=hex"" {_;}
	receive() payable external selector=hex"1" {}
	fallback() external selector=hex"abc" {}
	function i() internal selector = hex"ab_dd" {}
	function p() private selector = hex"ab_dd" {}
}
