value: bool

/// Constructor that initializes the `bool` value to the given `init_value`.
public
def __init__(initvalue: bool):
   value = initvalue;
}

/// A message that can be called on instantiated contracts.
/// This one flips the value of the stored `bool` from `true`
/// to `false` and vice versa.
public
def flip():
    value = !value;
}

public
def get() returns (test: bool):
    return value;
}