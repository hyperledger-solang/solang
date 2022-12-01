
        interface I {
            function X(bytes) external;
        }

        contract X {
            function x() public returns (bytes8) {
                return I.X.selector;
            }
        }