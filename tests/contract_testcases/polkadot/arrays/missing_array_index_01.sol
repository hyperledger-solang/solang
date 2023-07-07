
        contract c {
            function foo() public returns (uint8) {
                    uint8[4] memory bar = [ 1, 2, 3, 4, 5 ];

                    return bar[0];
            }
        }
// ---- Expect: diagnostics ----
// error: 4:43-60: conversion from uint8[5] to uint8[4] not possible
