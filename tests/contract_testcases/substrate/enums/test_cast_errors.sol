contract test {
            enum state { foo, bar, baz }
            function foo() public pure returns (uint8) {
                return state.foo;
            }
        }