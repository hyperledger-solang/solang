import { XdrPrimitiveType } from './xdr-type';
import { XdrWriterError } from './errors';

export class Float extends XdrPrimitiveType {
  /**
   * @inheritDoc
   */
  static read(reader) {
    return reader.readFloatBE();
  }

  /**
   * @inheritDoc
   */
  static write(value, writer) {
    if (typeof value !== 'number') throw new XdrWriterError('not a number');

    writer.writeFloatBE(value);
  }

  /**
   * @inheritDoc
   */
  static isValid(value) {
    return typeof value === 'number';
  }
}
