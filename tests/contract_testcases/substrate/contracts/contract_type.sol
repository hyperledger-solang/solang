
        contract printer {
            function test() public {
                print("In f.test()");
            }
        }

        contract foo {
            function test1(printer x) public {
                address y = x;
            }

            function test2(address x) public {
                printer y = printer(x);
            }
        }
// ---- Expect: diagnostics ----
// error: 10:29-30: implicit conversion to address from contract printer not allowed
