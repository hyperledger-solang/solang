
        contract foo {
            function test() public {
                int32[] memory a = new int32[](-1);

                assert(a.length == 5);
            }
        }
// ---- Expect: diagnostics ----
// error: 4:48-50: negative value -1 does not fit into type uint32. Cannot implicitly convert signed literal to unsigned type.
