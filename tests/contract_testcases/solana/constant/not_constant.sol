
        contract C {
            uint256 public constant STATIC = 42;
        }

        contract foo {
            function f() external returns (uint) {
                uint a = C.STATIC();
                return a;
            }
        }
        
// ---- Expect: diagnostics ----
// error: 8:26-36: a contract needs a program id to be called. Either a '@program_id' must be declared above a contract or the {program_id: ...} call argument must be present
