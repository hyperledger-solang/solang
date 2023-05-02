
        contract b {
            function step1(b other) public {
                this = other;
            }
        }
// ---- Expect: diagnostics ----
// error: 4:17-21: expression is not assignable
