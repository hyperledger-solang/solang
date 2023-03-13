struct A { B b; F f; }
struct B { C c; }
struct C { D d; }
struct D { A a; }
struct F { G g; }
struct G { H h; }
struct H { A a; }