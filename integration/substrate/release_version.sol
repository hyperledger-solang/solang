contract release {
    function print_then_error(int8 num) public returns (int8) {
        print("Hello!");
        int8 ovf = num + 120;
        return ovf;
    }
}
