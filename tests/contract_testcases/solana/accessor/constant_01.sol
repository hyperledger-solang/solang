
        contract x {
            bytes foo;
            bytes32 public constant z = keccak256(foo);
        }
// ---- Expect: diagnostics ----
// error: 4:51-54: cannot read contract variable 'foo' in constant expression
