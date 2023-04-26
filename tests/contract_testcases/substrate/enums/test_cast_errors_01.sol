contract test {
            enum state {  }
            function foo() public pure returns (uint8) {
                return state.foo;
            }
        }
// ----
// error (33-38): enum 'state' has no fields
// error (130-133): enum test.state does not have value foo
