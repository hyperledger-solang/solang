
        interface I {
            function X(bytes) external;
        }

        contract X {
            function x() public returns (bytes8) {
                return I.X.selector;
            }
        }
// ----
// warning (44-45): X is already defined as a contract name
// 	note (82-206): location of previous definition
// warning (107-143): function can be declared 'pure'
