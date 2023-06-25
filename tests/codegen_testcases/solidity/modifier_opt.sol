// RUN: --target substrate --release --emit cfg

contract Test {
    function testMethod() public payable m1 m2 returns (uint256) {
        return 2 ** 256 - 1;
        // CHECK: block0: # entry
        // CHECK: return uint256 -1
        // NOT-CHECK: return ((uint256 2 ** uint256 256) - uint256 1)
    }

    modifier m1() {
        _;
    }

    modifier m2() {
        _;
        // CHECK: block0: # entry
        // CHECK: branchcond (uint128((builtin Value ())) != uint128 -1), block1, block2
        // NOT-CHECK: branchcond (uint128((builtin Value ())) != (uint128 2 ** uint128 127)), block1, block2
        require(msg.value != 2 ** 128 - 1);
    }
}
