
        contract foo {
            enum foo2 { bar1, bar2 }
            function bar() public returns (foo2[10] storage x) {
            }
        }