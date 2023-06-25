// RUN: --target substrate --release --emit cfg
contract Test {
    modifier modified() {
        _;
    }

    function testMethod() public pure modified returns (uint256) {
        return 2 ** 256 - 1;
        // CHECK: block0: # entry
        // NOT-CHECK: return ((uint256 2 ** uint256 256) - uint256 1)
        // CHECK: return uint256 -1
    }
}
