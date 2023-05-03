
        contract foo {
            struct s {
                bool f1;
                int32 f2;
            }
            s[] bar;

            function test() public {
                s storage x = bar.pop();
            }
        }
// ---- Expect: diagnostics ----
// error: 10:35-38: conversion from struct foo.s to struct foo.s storage not possible
