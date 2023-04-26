
        contract x {
            bytes foo;
            bytes32 public constant z = keccak256(foo);
        }
// ----
// error (95-98): cannot read contract variable 'foo' in constant expression
