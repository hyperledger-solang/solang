
        contract a {
            function f1() public {}
        }

        contract b is a {
            function f2() public {
                super.f2();
            }
        }
// ---- Expect: diagnostics ----
// error: 8:23-25: unknown function or type 'f2'
