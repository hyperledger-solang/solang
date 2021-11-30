
        struct S {
            int32 f1;
            int32 f2;
        }

        function x(S storage x) view { x.f1 = 102; }
        