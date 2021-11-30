contract test {
            int64 foo = 1844674;

            function bar() public view {
                foo = 102;
            }
        }