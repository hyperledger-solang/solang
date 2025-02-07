import { Bool } from './bool';
import { XdrPrimitiveType } from './xdr-type';

export class Option extends XdrPrimitiveType {
  constructor(childType) {
    super();
    this._childType = childType;
  }

  /**
   * @inheritDoc
   */
  read(reader) {
    if (Bool.read(reader)) {
      return this._childType.read(reader);
    }

    return undefined;
  }

  /**
   * @inheritDoc
   */
  write(value, writer) {
    const isPresent = value !== null && value !== undefined;

    Bool.write(isPresent, writer);

    if (isPresent) {
      this._childType.write(value, writer);
    }
  }

  /**
   * @inheritDoc
   */
  isValid(value) {
    if (value === null || value === undefined) {
      return true;
    }
    return this._childType.isValid(value);
  }
}
