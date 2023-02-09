// RUN: --target solana --emit cfg
contract Concise {
	// CHECK: params: address owner,int64 bar
	// CHECK: returns: uint256 balance
	// CHECK: ty:address %owner = (arg #0)
	mapping(address owner => mapping(int64 bar => uint256 balance) foo) public balanceOf;
}
