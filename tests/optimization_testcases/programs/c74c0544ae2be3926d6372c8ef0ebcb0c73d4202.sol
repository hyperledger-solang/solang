contract Testing {
    function testExternalFunction(
        bytes memory buffer
    ) public view returns (bytes8, address) {
        function(uint8) external returns (int8) fPtr = abi.decode(
            buffer,
            (function(uint8) external returns (int8))
        );
        return (fPtr.selector, fPtr.address);
    }
}
