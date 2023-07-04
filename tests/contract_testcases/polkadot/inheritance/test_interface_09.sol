
        interface bar {
            function f1(address a) external;
        }

        interface bar2 {
            function f1(address a) external;
        }

        contract x is bar {
            function f1(address a) public {}
        }

        contract y is bar2, x {
            function f2(address a) public {}
        }
        
// ---- Expect: diagnostics ----
// warning: 11:13-42: function can be declared 'pure'
// warning: 11:33-34: function parameter 'a' is unused
// warning: 15:13-42: function can be declared 'pure'
// warning: 15:33-34: function parameter 'a' is unused
