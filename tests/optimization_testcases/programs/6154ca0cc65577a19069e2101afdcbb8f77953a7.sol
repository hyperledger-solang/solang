contract foo {
    mapping(int64 => uint64) public f1;

    constructor() {
        f1[2000] = 1;
        f1[4000] = 2;
    }
}
