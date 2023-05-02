
        contract printer {
            function test() public {
                bytes x = abi.encodeWithSelector();
            }
        }
// ---- Expect: diagnostics ----
// error: 4:27-51: function requires one 'bytes4' selector argument
