contract felix {
    enum Felines {
        None,
        Lynx,
        Felis,
        Puma,
        Catopuma
    }
    Felines[100] group_a;
    Felines[100] group_b;

    function count_pumas(Felines[100] storage cats) private returns (uint32) {
        uint32 count = 0;
        uint32 i = 0;

        for (i = 0; i < cats.length; i++) {
            if (cats[i] == Felines.Puma) {
                ++count;
            }
        }

        return count;
    }

    function all_pumas() public returns (uint32) {
        Felines[100] storage ref = group_a;

        uint32 total = count_pumas(ref);

        ref = group_b;

        total += count_pumas(ref);

        return total;
    }
}
