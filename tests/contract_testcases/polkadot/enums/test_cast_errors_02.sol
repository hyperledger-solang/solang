contract test {
            enum state { foo, bar, baz }
            function foo() public pure returns (uint8) {
                return uint8(state.foo);
            }
        }
// ---- Expect: diagnostics ----
