
        contract base {
            constructor(uint64 a) public {}
        }

        contract apex is base(true) {
            function foo(uint64 a) virtual internal returns (uint64) {
                return a + 102;
            }
        }
// ---- Expect: diagnostics ----
// warning: 3:35-41: 'public': visibility for constructors is ignored
// error: 6:9-10:10: missing arguments to base contract 'base' constructor
// error: 6:31-35: conversion from bool to uint64 not possible
