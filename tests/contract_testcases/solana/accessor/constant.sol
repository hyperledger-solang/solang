
        contract x {
            bytes32 public constant z = blockhash(1);
        }
// ---- Expect: diagnostics ----
// error: 3:41-53: cannot call function in constant expression
