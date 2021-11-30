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
        