contract b {
    function testPtr(int a) public pure returns (int) {
        return a/2;
    }
}

contract testing is b {
    function(int) external pure returns (int) sfPtr = this.testPtr;
    function(int) internal pure returns (int) sgPtr = testPtr;
}

// ---- Expect: diagnostics ----
// warning: 8:5-67: storage variable 'sfPtr' has been assigned, but never read
// warning: 9:5-62: storage variable 'sgPtr' has been assigned, but never read
