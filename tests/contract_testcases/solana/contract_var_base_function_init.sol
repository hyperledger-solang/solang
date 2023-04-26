contract b {
    function testPtr(int a) public pure returns (int) {
        return a/2;
    }
}

contract testing is b {
    function(int) external pure returns (int) sfPtr = this.testPtr;
    function(int) internal pure returns (int) sgPtr = testPtr;
}

// ----
// warning (126-188): storage variable 'sfPtr' has been assigned, but never read
// warning (194-251): storage variable 'sgPtr' has been assigned, but never read
