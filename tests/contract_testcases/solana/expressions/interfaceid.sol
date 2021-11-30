
        contract foo {
            function get() public returns (bytes4) {
                return type(foo).interfaceId;
            }
        }