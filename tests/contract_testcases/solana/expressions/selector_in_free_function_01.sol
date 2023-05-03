
        interface I {
            function X(bytes) external;
        }

        contract X {
            function x() public returns (bytes8) {
                return I.X.selector;
            }
        }
// ---- Expect: diagnostics ----
// warning: 3:22-23: X is already defined as a contract name
// 	note 6:9-10:10: location of previous definition
// warning: 7:13-49: function can be declared 'pure'
