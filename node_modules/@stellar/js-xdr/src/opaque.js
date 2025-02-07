import { XdrCompositeType } from './xdr-type';
import { XdrWriterError } from './errors';

export class Opaque extends XdrCompositeType {
  constructor(length) {
    super();
    this._length = length;
  }

  /**
   * @inheritDoc
   */
  read(reader) {
    return reader.read(this._length);
  }

  /**
   * @inheritDoc
   */
  write(value, writer) {
    const { length } = value;
    if (length !== this._length)
      throw new XdrWriterError(
        `got ${value.length} bytes, expected ${this._length}`
      );
    writer.write(value, length);
  }

  /**
   * @inheritDoc
   */
  isValid(value) {
    return Buffer.isBuffer(value) && value.length === this._length;
  }
}
