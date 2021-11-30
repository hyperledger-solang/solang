
        contract x {
            bytes foo;
            bytes32 public constant z = keccak256(foo);
        }