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
        
// ----
// warning (187-207): function can be declared 'view'
// warning (274-282): local variable 'variable' has been assigned, but never read
// warning (391-400): local variable 'func_type' has been assigned, but never read
