contract test {
            enum state {  }
            function foo() public pure returns (uint8) {
                return state.foo;
            }
        }
// ---- Expect: diagnostics ----
// error: 2:18-23: enum 'state' has no fields
// error: 4:30-33: enum test.state does not have value foo
