import { UnsignedInt } from './unsigned-int';
import { XdrCompositeType } from './xdr-type';
import { XdrReaderError, XdrWriterError } from './errors';

export class String extends XdrCompositeType {
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
        `saw ${size} length String, max allowed is ${this._maxLength}`
      );

    return reader.read(size);
  }

  readString(reader) {
    return this.read(reader).toString('utf8');
  }

  /**
   * @inheritDoc
   */
  write(value, writer) {
    // calculate string byte size before writing
    const size =
      typeof value === 'string'
        ? Buffer.byteLength(value, 'utf8')
        : value.length;
    if (size > this._maxLength)
      throw new XdrWriterError(
        `got ${value.length} bytes, max allowed is ${this._maxLength}`
      );
    // write size info
    UnsignedInt.write(size, writer);
    writer.write(value, size);
  }

  /**
   * @inheritDoc
   */
  isValid(value) {
    if (typeof value === 'string') {
      return Buffer.byteLength(value, 'utf8') <= this._maxLength;
    }
    if (value instanceof Array || Buffer.isBuffer(value)) {
      return value.length <= this._maxLength;
    }
    return false;
  }
}
