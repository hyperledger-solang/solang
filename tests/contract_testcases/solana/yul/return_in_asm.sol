contract Contract {
    constructor() {
        assembly ("memory-safe") {
            return(0, 0)
        }
    }
}

// ---- Expect: diagnostics ----
// error: 3:19-32: flag 'memory-safe' not supported
// error: 4:13-25: builtin 'return' is not available for target solana. Please, open a GitHub issue at https://github.com/hyperledger/solang/issues if there is need to support this function
