contract Testing {
    struct NonConstantStruct {
        string[] b;
    }

    string[] vec_2;
    NonConstantStruct[] public complex_array;

    function fn1() public {
        vec_2.push("tea");
    }

    function fn2() public {
        vec_2.push("coffee");
    }

    function fn3() public {
        NonConstantStruct memory ss = NonConstantStruct(vec_2);
        complex_array.push(ss);
    }

    function fn4() public {
        vec_2.pop();
    }

    function fn5() public {
        vec_2.pop();
    }

    function fn6() public {
        vec_2.push("cortado");
    }

    function fn7() public {
        vec_2.push("cappuccino");
    }

    function fn8() public {
        NonConstantStruct memory sr = NonConstantStruct(vec_2);
        complex_array.push(sr);
    }

    function clear() public {
        vec_2 = new string[](0);
        complex_array = new NonConstantStruct[](0);
    }
}
