contract Flip {
    function flip () pure public {
	print("flip");
    }
}

contract Inc {
    Flip _flipper;

    constructor (Flip _flipperContract) {
	_flipper = _flipperContract;
    }

    function superFlip () pure public {
	_flipper.flip();
    }
}
