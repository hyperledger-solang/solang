
        contract b {
            function step1(b other) public {
                this = other;
            }
        }
// ----
// error (83-87): expression is not assignable
