contract foo {
    int64[4][2] public f1;

    constructor() {
        f1[1][0] = 4;
        f1[1][1] = 3;
        f1[1][2] = 2;
        f1[1][3] = 1;
    }
}
