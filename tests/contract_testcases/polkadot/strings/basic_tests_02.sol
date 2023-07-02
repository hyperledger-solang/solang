
        contract c {
            function foo() public {
                    string f = new bytes(2);
            }
        }
// ---- Expect: diagnostics ----
// error: 4:32-44: conversion from bytes to string not possible
