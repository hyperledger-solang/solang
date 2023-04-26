
        contract superior {
            function test() public {
                inferior i = new inferior();

            (bytes x, bool y) = address(i).call(hex"1222");
            }
        }

        contract inferior {
            function baa() public {
                print("Baa!");
            }
        }
// ----
// error (125-132): conversion from bool to bytes not possible
