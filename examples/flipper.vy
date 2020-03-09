value: bool

@public
def __init__(initvalue: bool):
    value = initvalue

@public
def flip():
    value = not value

@public
def get() -> (bool):
    return value
