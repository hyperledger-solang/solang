
        contract I {
            function X() external {}
        }

        contract foo {
            function f(I t) public returns (bytes8) {
                return t.X.selector;
            }
        }
        