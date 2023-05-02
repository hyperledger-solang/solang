
        contract c {
            function foo() public {
                    bytes f = new string(2);
            }
        }
// ---- Expect: diagnostics ----
// error: 4:31-44: conversion from string to bytes not possible
