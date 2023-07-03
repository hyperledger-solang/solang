
        contract c {
            modifier foo(int32 f) { _; }

            function bar(bool x) foo(x) public {}
        }
// ---- Expect: diagnostics ----
// warning: 3:32-33: function parameter 'f' is unused
// error: 5:38-39: conversion from bool to int32 not possible
