contract b {
    function testPtr(int a) public pure returns (int) {
        return a/2;
    }
}

contract testing is b {
    function(int) external pure returns (int) sfPtr = this.testPtr;
    function(int) internal pure returns (int) sgPtr = testPtr;
}
