
        contract foo {
            function get() public returns (bytes4) {
                return type(foo).interfaceId;
            }
        }
// ----
// error (100-121): type(…).interfaceId is permitted on interface, not contract foo
