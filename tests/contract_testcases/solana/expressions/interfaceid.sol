
        contract foo {
            function get() public returns (bytes4) {
                return type(foo).interfaceId;
            }
        }
// ---- Expect: diagnostics ----
// error: 4:24-45: type(…).interfaceId is permitted on interface, not contract foo
