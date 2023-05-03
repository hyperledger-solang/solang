contract Fptr {
    struct S { int ff; function (S memory) external fptr; }
    function func(S memory) public pure {}
}


// ---- Expect: diagnostics ----
