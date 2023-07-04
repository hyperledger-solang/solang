
        interface IFoo {
            function bar(uint32) external pure returns (uint32);
        }

        contract foo is IFoo {
            function bar(uint32 a) public pure returns (uint32) {
                return a * 2;
            }
        }
// ---- Expect: diagnostics ----
