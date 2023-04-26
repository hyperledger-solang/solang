contract Contract {
    constructor() {
        assembly ("memory-safe") {
            return(0, 0)
        }
    }
}

// ----
// error (58-71): flag 'memory-safe' not supported
// error (87-99): builtin 'return' is not available for target solana. Please, open a GitHub issue at https://github.com/hyperledger/solang/issues if there is need to support this function
