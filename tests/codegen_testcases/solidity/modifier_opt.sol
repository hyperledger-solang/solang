// RUN: --target polkadot --emit cfg

contract Test {
    // BEGIN-CHECK: Test::testMethod::modifier0::m1
    modifier m1() {
        require(msg.value != 1 - 1);
        _;
        require(msg.value != 2 ** 128 - 1);

        // CHECK: block0: # entry
        // CHECK: branchcond (uint128((builtin Value ())) != uint128 0),
        // NOT-CHECK: uint128 1 - uint128 1

        // CHECK: branchcond (uint128((builtin Value ())) != uint128 340282366920938463463374607431768211455),
        // NOT-CHECK: uint128 2 ** uint128 127
    }

    // BEGIN-CHECK: Test::testMethod::modifier1::m2
    modifier m2() {
        require(msg.value != 2 ** 128 - 1);
        _;
        require(msg.value != 1 - 1);

        // CHECK: block0: # entry
        // CHECK: branchcond (uint128((builtin Value ())) != uint128 340282366920938463463374607431768211455),
        // NOT-CHECK: uint128 2 ** uint128 127,

        // CHECK: branchcond (uint128((builtin Value ())) != uint128 0),
        // NOT-CHECK: uint128 1 - uint128 1
    }

    // BEGIN-CHECK: Test::function::testMethod
    function testMethod() public payable m1 m2 returns (uint256) {
        return 2 ** 256 - 1;

        // CHECK: block0: # entry
        // CHECK: return uint256 115792089237316195423570985008687907853269984665640564039457584007913129639935
        // NOT-CHECK: (uint256 2 ** uint256 256) - uint256 1
    }
}
