pragma solidity ^0.4.25;

contract test {
	function foo() public pure returns (int) {
		int a = b + 3;
		int b = a + 7;

		return a * b;
	}
}
