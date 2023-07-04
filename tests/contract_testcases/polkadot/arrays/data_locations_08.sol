
        contract foo {
            enum foo2 { bar1, bar2 }
            function bar() public returns (foo2[10] storage x) {
            }
        }
// ---- Expect: diagnostics ----
// error: 4:53-60: return type of type 'storage' not allowed public or external functions
