
        contract foo {
            function get() public returns (bytes4) {
                assembly {
                    let returndata_size := mload(0x40)
                    revert(add(32, 0x40), returndata_size)
                }
            }
        }
// ----
// error (147-158): builtin 'mload' is not available for target solana. Please, open a GitHub issue at https://github.com/hyperledger/solang/issues if there is need to support this function
