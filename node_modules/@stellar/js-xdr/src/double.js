import { XdrPrimitiveType } from './xdr-type';
import { XdrWriterError } from './errors';

export class Double extends XdrPrimitiveType {
  /**
   * @inheritDoc
   */
  static read(reader) {
    return reader.readDoubleBE();
  }

  /**
   * @inheritDoc
   */
  static write(value, writer) {
    if (typeof value !== 'number') throw new XdrWriterError('not a number');

    writer.writeDoubleBE(value);
  }

  /**
   * @inheritDoc
   */
  static isValid(value) {
    return typeof value === 'number';
  }
}
