
        contract C {
            struct E {
                bytes4 b4;
            }
            struct S {
                int64 f1;
                bool f2;
                E f3;
            }
            S public a = S({f1: -63, f2: false, f3: E("nuff")});
            S[100] public s;
            mapping(int => S) public m;
            E public constant e = E("cons");

            constructor() {
                s[99] = S({f1: 65535, f2: true, f3: E("naff")});
                m[1023413412] = S({f1: 414243, f2: true, f3: E("niff")});
            }

            function f() public view {
                (int64 a1, bool b, E memory c) = this.a();
                require(a1 == -63 && !b && c.b4 == "nuff", "a");
                (a1, b, c) = this.s(99);
                require(a1 == 65535 && b && c.b4 == "naff", "b");
                (a1, b, c) = this.m(1023413412);
                require(a1 == 414243 && b && c.b4 == "niff", "c");
                c.b4 = this.e();
                require(a1 == 414243 && b && c.b4 == "cons", "E");
            }
        }