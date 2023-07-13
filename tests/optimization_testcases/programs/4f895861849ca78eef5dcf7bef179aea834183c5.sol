
    contract testing {

    function testPtr(int a) public pure returns (int, int) {
        return (a/2, 3);
    }

    function doTest() public view returns (int, int) {
    function(int) external pure returns (int, int) sfPtr = this.testPtr;

       (int a, int b) = sfPtr(2);
       return (a, b);
    }
}
    