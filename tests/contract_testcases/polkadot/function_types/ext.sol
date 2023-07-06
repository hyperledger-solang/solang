contract test {
            function x(int64 arg1) internal returns (bool) {
                function(int32) external returns (bool) x = foo;
            }

            function foo(int32) public returns (bool) {
                return false;
            }
        }
// ---- Expect: diagnostics ----
// error: 3:61-64: conversion from function(int32) internal returns (bool) to function(int32) external returns (bool) not possible
