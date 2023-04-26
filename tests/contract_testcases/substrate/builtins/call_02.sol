
        contract superior {
            function test() public {
                inferior i = new inferior();

                bytes x = address(i).call(hex"1222");
            }
        }

        contract inferior {
            function baa() public {
                print("Baa!");
            }
        }
// ----
// error (138-164): destucturing statement needed for function that returns multiple values
