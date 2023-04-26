
        contract foo {
            function test() public {
                int32[] memory a = new int32[](2);
                int32 i = 1;

                a[i] = 5;
            }
        }
// ----
// error (158-162): array subscript must be an unsigned integer, not 'int32'
