
        interface I {
            function X(bytes) external;
        }

        function x() returns (bytes8) {
            return I.X.selector;
        }

        contract foo {}
        
// ---- Expect: diagnostics ----
// warning: 6:9-38: function can be declared 'pure'
