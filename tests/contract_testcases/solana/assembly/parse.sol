
        contract foo {
            function get() public returns (bytes4) {
                assembly {
                    let returndata_size := mload(returndata)
                    revert(add(32, returndata), returndata_size)
                }
            }
        }