
contract testing  {
    function test_address() public view returns (uint256 ret) {
        assembly {
            let a := address()
            ret := a
        }
    }

    function test_balance() public view returns (uint256 ret) {
        assembly {
            let a := address()
            ret := balance(a)
        }
    }

    function test_selfbalance() public view returns (uint256 ret) {
        assembly {
            let a := selfbalance()
            ret := a
        }
    }
}