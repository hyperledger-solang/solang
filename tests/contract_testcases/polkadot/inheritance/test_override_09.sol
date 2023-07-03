
        interface b {
                function bar(int64 x) external;
        }

        contract a is b {
                function bar(int64 x) public override;
        }
        
// ---- Expect: diagnostics ----
// error: 6:9-8:10: contract 'a' missing override for function 'bar'
// 	note 3:17-47: declaration of function 'bar'
// error: 7:17-54: function with no body missing 'virtual'. This was permitted in older versions of the Solidity language, please update.
