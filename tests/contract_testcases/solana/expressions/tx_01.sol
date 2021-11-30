
        contract foo {
            function test() public pure returns (uint64) {
                return tx.gasprice;
            }
        }