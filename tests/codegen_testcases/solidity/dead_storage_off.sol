// RUN: --no-dead-storage --emit cfg --target polkadot
contract nodeadstorage {
    int a;

    // simple test. Two references to "a" must result in loadstorage twice with --no-dead-storage flag

// BEGIN-CHECK: nodeadstorage::function::test1
	function test1() public view returns (int) {
        return a + a;
	}
// CHECK: load storage slot(uint256 0) ty:int256
// CHECK: load storage slot(uint256 0) ty:int256

}
