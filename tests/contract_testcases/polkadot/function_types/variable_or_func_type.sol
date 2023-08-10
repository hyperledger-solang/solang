contract bar {
            int public total;
        }

        interface I {
            function total() external returns (int);
        }

        contract foo is I, bar {
            function f1() public {
                // now we want the variable
                int variable = total;
                // now we want the function type
                function() internal returns (int) func_type = total;
            }
        }
        
// ---- Expect: diagnostics ----
// warning: 10:13-33: function can be declared 'view'
// warning: 12:21-29: local variable 'variable' is unused
// warning: 14:51-60: local variable 'func_type' is unused
