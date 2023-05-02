
        contract printer {
            function test() public {
                bytes x = abi.encodeWithSignature();
            }
        }
// ---- Expect: diagnostics ----
// error: 4:27-52: function requires one 'string' signature argument
