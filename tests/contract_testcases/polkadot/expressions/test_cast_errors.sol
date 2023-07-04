contract test {
            function foo(uint bar) public {
                bool is_nonzero = bar;
            }
        }
// ---- Expect: diagnostics ----
// error: 3:35-38: conversion from uint256 to bool not possible
