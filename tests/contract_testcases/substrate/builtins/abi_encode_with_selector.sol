
        contract printer {
            function test() public {
                bytes x = abi.encodeWithSelector();
            }
        }
// ----
// error (91-115): function requires one 'bytes4' selector argument
