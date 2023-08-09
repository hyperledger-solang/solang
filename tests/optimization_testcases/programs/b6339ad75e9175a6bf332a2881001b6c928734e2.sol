contract testing {
    struct stru_test {
        string a;
        uint b;
    }

    stru_test ss1; // slot: 16
    stru_test ss2; // slot: 56

    function test_slot() public view returns (uint256) {
        uint256 ret = 98;
        stru_test storage local_t = ss2;
        assembly {
            let a := ss1.slot
            let b := mul(local_t.slot, 1000)
            ret := add(a, b)
            // offset should always be zero
            ret := sub(ret, ss2.offset)
            ret := sub(ret, local_t.offset)
        }

        return ret;
    }

    function call_data_array(
        uint32[] calldata vl
    ) public pure returns (uint256, uint256) {
        uint256 ret1 = 98;
        uint256 ret2 = 99;
        assembly {
            let a := vl.offset
            let b := vl.length
            ret1 := a
            ret2 := b
        }

        return (ret1, ret2);
    }

    function selector_address() public view returns (uint256, uint256) {
        function() external returns (uint256) fPtr = this.test_slot;
        uint256 ret1 = 256;
        uint256 ret2 = 129;
        assembly {
            ret1 := fPtr.address
            ret2 := fPtr.selector
        }

        return (ret1, ret2);
    }
}
