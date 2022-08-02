// RUN: --target solana --emit cfg

contract C {
    // BEGIN-CHECK: C::C::function::combineToFunctionPointer__address_bytes4
    function combineToFunctionPointer(address newAddress, bytes4 newSelector) public pure returns (bytes4, address) {
        function() external fun;
        assembly {
            // CHECK: store (struct %fun field 1), uint32((arg #1))
            fun.selector := newSelector
            // CHECK: store (struct %fun field 0), (arg #0)
            fun.address  := newAddress
        }

        return (fun.selector, fun.address);
    }
}
