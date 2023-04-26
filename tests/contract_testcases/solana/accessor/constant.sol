
        contract x {
            bytes32 public constant z = blockhash(1);
        }
// ----
// error (62-74): cannot call function in constant expression
