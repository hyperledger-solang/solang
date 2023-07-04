
        contract printer {
            function test() public {
                print("In f.test()");
            }
        }

        contract foo {
            function test1(printer x) public {
                address y = 102;
            }
        }
// ---- Expect: diagnostics ----
// error: 10:29-32: expected 'address', found integer
