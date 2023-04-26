
        contract a {
            function f1() public {}
        }

        contract b is a {
            function f2() public {
                super.f2();
            }
        }
// ----
// error (152-154): unknown function or type 'f2'
