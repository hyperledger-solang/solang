
        contract a {
            int64 public x = 3;
            function f() virtual payable external {
                x = 1;
            }

            function f() override payable external {
                x = 2;
            }
        }