contract test {
            enum state {  }
            function foo() public pure returns (uint8) {
                return state.foo;
            }
        }