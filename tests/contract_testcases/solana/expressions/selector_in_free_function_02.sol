
        contract I {
            function X() external {}
        }

        contract foo {
            function f(I t) public returns (bytes4) {
                return t.X.selector;
            }
        }
        