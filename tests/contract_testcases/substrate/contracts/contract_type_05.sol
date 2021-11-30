
        contract printer {
            function test() public returns (printer) {
                return new printer({});
            }
        }