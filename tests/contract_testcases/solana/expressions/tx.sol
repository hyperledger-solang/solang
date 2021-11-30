
        contract foo {
            function test() public pure returns (address) {
                return tx.origin;
            }
        }