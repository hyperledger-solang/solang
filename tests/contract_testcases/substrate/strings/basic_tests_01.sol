
        contract c {
            function foo() public {
                    bytes f = new string(2);
            }
        }
// ----
// error (88-101): conversion from string to bytes not possible
