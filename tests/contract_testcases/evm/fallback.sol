
function fallback() pure returns (int) {
	return 1;
}

function receive(int a) pure returns (int) {
	return 2;
}

contract C {
	function fallback() pure public {}
	function receive() pure public {}

	fallback() external returns (int) {}
	receive(int) external {}
}

// ---- Expect: diagnostics ----
// warning: 2:1-39: function named fallback is not the fallback function of the contract. Remove the function keyword to define the fallback function
// warning: 6:1-43: function named receive is not the receive function of the contract. Remove the function keyword to define the receive function
// warning: 11:2-33: function named fallback is not the fallback function of the contract. Remove the function keyword to define the fallback function
// warning: 11:11-19: fallback is already defined as a function
// 	note 2:10-18: location of previous definition
// warning: 12:2-32: function named receive is not the receive function of the contract. Remove the function keyword to define the receive function
// warning: 12:11-18: receive is already defined as a function
// 	note 6:10-17: location of previous definition
// error: 14:2-35: fallback can be defined as "fallback()" or alternatively as "fallback(bytes calldata) returns (bytes memory)"
// error: 15:2-23: receive function cannot have parameters
