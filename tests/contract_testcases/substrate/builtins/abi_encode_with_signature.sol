
        contract printer {
            function test() public {
                bytes x = abi.encodeWithSignature();
            }
        }
// ----
// error (91-116): function requires one 'string' signature argument
