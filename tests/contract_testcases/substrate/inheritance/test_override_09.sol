
        interface b {
                function bar(int64 x) external;
        }

        contract a is b {
                function bar(int64 x) public override;
        }
        
// ----
// error (90-172): contract 'a' missing override for function 'bar'
// 	note (39-69): declaration of function 'bar'
// error (124-161): function with no body missing 'virtual'. This was permitted in older versions of the Solidity language, please update.
