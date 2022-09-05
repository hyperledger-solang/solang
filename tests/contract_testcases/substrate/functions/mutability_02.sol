abstract contract test {
            function bar(int64[] storage foo) private pure returns (int64) {
                return foo[0];
            }
        }