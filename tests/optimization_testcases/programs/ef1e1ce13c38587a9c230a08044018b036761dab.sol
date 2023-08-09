contract foo {
    int[] arr;

    function f() public returns (int, int) {
        int[] storage ptrArr = arr;
        ptrArr.push(1);
        ptrArr.push(2);
        (int a, int b) = (ptrArr[0], ptrArr[1]);
        return (a, b);
    }
}
