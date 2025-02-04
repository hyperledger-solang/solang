import { UnsignedInt } from './unsigned-int';
import { XdrCompositeType } from './xdr-type';
import { XdrReaderError, XdrWriterError } from './errors';

export class VarOpaque extends XdrCompositeType {
  constructor(maxLength = UnsignedInt.MAX_VALUE) {
    super();
    this._maxLength = maxLength;
  }

  /**
   * @inheritDoc
   */
  read(reader) {
    const size = UnsignedInt.read(reader);
    if (size > this._maxLength)
      throw new XdrReaderError(
        `saw ${size} length VarOpaque, max allowed is ${this._maxLength}`
      );
    return reader.read(size);
  }

  /**
   * @inheritDoc
   */
  write(value, writer) {
    const { length } = value;
    if (value.length > this._maxLength)
      throw new XdrWriterError(
        `got ${value.length} bytes, max allowed is ${this._maxLength}`
      );
    // write size info
    UnsignedInt.write(length, writer);
    writer.write(value, length);
  }

  /**
   * @inheritDoc
   */
  isValid(value) {
    return Buffer.isBuffer(value) && value.length <= this._maxLength;
  }
}
