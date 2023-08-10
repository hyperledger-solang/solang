contract foo {
    function return_address() public returns (address) {
        return address'CXQw5tfeRKKzV4hk6PcdyKyANSvFxoZCKwHkVXAhAYSJ';
    }

    function address_arg(address a) public {
        assert(a == address'66Eh1STPJgabub73TP8YbN7VNCwjaVTEJGHRxCLeBJ4A');
    }
}
