
        contract c {
            function foo() public {
                    string f = new bytes(2);
            }
        }
// ----
// error (89-101): conversion from bytes to string not possible
