contract test {
            function bar(int[] storage foo) internal view {
                foo[0] = 102;
            }
        }