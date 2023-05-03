
        contract Math {
            enum MathError {
                NO_ERROR
            }
        }

        contract IsMath is Math {
            struct WithMath {
                MathError math;
            }
        }
    
// ---- Expect: diagnostics ----
