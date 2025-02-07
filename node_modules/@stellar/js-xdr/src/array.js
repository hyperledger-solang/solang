import { XdrCompositeType } from './xdr-type';
import { XdrWriterError } from './errors';

export class Array extends XdrCompositeType {
  constructor(childType, length) {
    super();
    this._childType = childType;
    this._length = length;
  }

  /**
   * @inheritDoc
   */
  read(reader) {
    // allocate array of specified length
    const result = new global.Array(this._length);
    // read values
    for (let i = 0; i < this._length; i++) {
      result[i] = this._childType.read(reader);
    }
    return result;
  }

  /**
   * @inheritDoc
   */
  write(value, writer) {
    if (!global.Array.isArray(value))
      throw new XdrWriterError(`value is not array`);

    if (value.length !== this._length)
      throw new XdrWriterError(
        `got array of size ${value.length}, expected ${this._length}`
      );

    for (const child of value) {
      this._childType.write(child, writer);
    }
  }

  /**
   * @inheritDoc
   */
  isValid(value) {
    if (!(value instanceof global.Array) || value.length !== this._length) {
      return false;
    }

    for (const child of value) {
      if (!this._childType.isValid(child)) return false;
    }
    return true;
  }
}
