
        contract foo {
            function get() public returns (bytes4) {
                assembly {
                    let returndata_size := mload(0x40)
                    revert(add(32, 0x40), returndata_size)
                }
            }
        }
// ---- Expect: diagnostics ----
// error: 5:44-55: builtin 'mload' is not available for target Solana. Please, open a GitHub issue at https://github.com/hyperledger-solang/solang/issues if there is need to support this function
