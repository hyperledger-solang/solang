export class XdrWriterError extends TypeError {
  constructor(message) {
    super(`XDR Write Error: ${message}`);
  }
}

export class XdrReaderError extends TypeError {
  constructor(message) {
    super(`XDR Read Error: ${message}`);
  }
}

export class XdrDefinitionError extends TypeError {
  constructor(message) {
    super(`XDR Type Definition Error: ${message}`);
  }
}

export class XdrNotImplementedDefinitionError extends XdrDefinitionError {
  constructor() {
    super(
      `method not implemented, it should be overloaded in the descendant class.`
    );
  }
}
