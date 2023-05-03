
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
// ---- Expect: diagnostics ----
// error: 6:14-21: conversion from bool to bytes not possible
