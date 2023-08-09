contract foo is bar {
    bytes2 public f1 = "ab";
}

interface bar {
    function f1() external returns (bytes2);
}
