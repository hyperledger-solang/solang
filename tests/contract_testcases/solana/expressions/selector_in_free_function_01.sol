
        interface I {
            function X(bytes) external;
        }

        contract X {
            function x() public returns (bytes4) {
                return I.X.selector;
            }
        }