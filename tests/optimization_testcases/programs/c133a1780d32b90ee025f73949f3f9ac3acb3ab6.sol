contract C {
    function myFun() public {}

    function test(
        uint256 newAddress,
        bytes8 newSelector
    ) public view returns (bytes8, address) {
        function() external fun = this.myFun;
        address myAddr = address(newAddress);
        assembly {
            fun.selector := newSelector
            fun.address := myAddr
        }

        return (fun.selector, fun.address);
    }
}
