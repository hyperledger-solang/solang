contract test {
            enum state { foo, bar, baz }
            function foo() public pure returns (uint8) {
                return state.foo;
            }
        }
// ---- Expect: diagnostics ----
// error: 4:24-33: implicit conversion from enum test.state to uint8 not allowed
