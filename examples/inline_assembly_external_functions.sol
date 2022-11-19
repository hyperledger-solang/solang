contract foo {
    function sum(uint64 a, uint64 b) public pure returns (uint64) {
        return a + b;
    }

    function bar() public view {
        function (uint64, uint64) external returns (uint64) fPtr = this.sum;
        assembly {
            // 'a' contains 'sum' selector
            let a := fPtr.selector

            // 'b' contains 'sum' address
            let b := fPtr.address
        }
    }
}
