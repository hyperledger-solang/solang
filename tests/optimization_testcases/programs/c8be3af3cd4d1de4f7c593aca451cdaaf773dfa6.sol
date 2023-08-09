contract deadstorage {
    uint public maxlen = 10000;
    uint public z;
    uint public v;

    constructor() {
        for (uint i = 0; i < 10; i++) {
            uint x = i * (10e34 + 9999);
            print("x:{}".format(x));
            v = x % maxlen;
            print("v:{}".format(v));
            z = v % maxlen;
            print("z:{}".format(z));
        }
    }
}
