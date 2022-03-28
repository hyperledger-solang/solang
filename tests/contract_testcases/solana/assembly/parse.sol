
        contract foo {
            function get() public returns (bytes4) {
                assembly {
                    let returndata_size := mload(0x40)
                    revert(add(32, 0x40), returndata_size)
                }
            }
        }