import { UnsignedInt } from './unsigned-int';
import { XdrCompositeType } from './xdr-type';
import { XdrReaderError, XdrWriterError } from './errors';

export class VarArray extends XdrCompositeType {
  constructor(childType, maxLength = UnsignedInt.MAX_VALUE) {
    super();
    this._childType = childType;
    this._maxLength = maxLength;
  }

  /**
   * @inheritDoc
   */
  read(reader) {
    const length = UnsignedInt.read(reader);
    if (length > this._maxLength)
      throw new XdrReaderError(
        `saw ${length} length VarArray, max allowed is ${this._maxLength}`
      );

    const result = new Array(length);
    for (let i = 0; i < length; i++) {
      result[i] = this._childType.read(reader);
    }
    return result;
  }

  /**
   * @inheritDoc
   */
  write(value, writer) {
    if (!(value instanceof Array))
      throw new XdrWriterError(`value is not array`);

    if (value.length > this._maxLength)
      throw new XdrWriterError(
        `got array of size ${value.length}, max allowed is ${this._maxLength}`
      );

    UnsignedInt.write(value.length, writer);
    for (const child of value) {
      this._childType.write(child, writer);
    }
  }

  /**
   * @inheritDoc
   */
  isValid(value) {
    if (!(value instanceof Array) || value.length > this._maxLength) {
      return false;
    }
    for (const child of value) {
      if (!this._childType.isValid(child)) return false;
    }
    return true;
  }
}
