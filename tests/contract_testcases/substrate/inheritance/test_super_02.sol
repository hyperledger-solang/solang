
        contract a {
            function f1() public {}
        }

        contract b is a {
            function f2() public {
                super.f2();
            }
        }