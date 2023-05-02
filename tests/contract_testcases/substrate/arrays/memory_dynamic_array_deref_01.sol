
        contract foo {
            function test() public {
                int32[] memory a = new int32[](2);
                int32 i = 1;

                a[i] = 5;
            }
        }
// ---- Expect: diagnostics ----
// error: 7:17-21: array subscript must be an unsigned integer, not 'int32'
