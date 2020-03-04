value: bool

@public
def __init__(initvalue: bool):
    self.value = initvalue

@public
def flip():
    self.value = not self.value

@public
@constant
def get() -> bool:
    return self.value