
        contract foo {
            function test() public {
                int32[] memory a = new int32[](2);

                a[-1] = 5;
            }
        }
// ---- Expect: diagnostics ----
// error: 6:19-21: negative value -1 does not fit into type uint32. Cannot implicitly convert signed literal to unsigned type.
