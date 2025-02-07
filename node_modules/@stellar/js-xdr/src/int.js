import { XdrPrimitiveType } from './xdr-type';
import { XdrWriterError } from './errors';

const MAX_VALUE = 2147483647;
const MIN_VALUE = -2147483648;

export class Int extends XdrPrimitiveType {
  /**
   * @inheritDoc
   */
  static read(reader) {
    return reader.readInt32BE();
  }

  /**
   * @inheritDoc
   */
  static write(value, writer) {
    if (typeof value !== 'number') throw new XdrWriterError('not a number');

    if ((value | 0) !== value) throw new XdrWriterError('invalid i32 value');

    writer.writeInt32BE(value);
  }

  /**
   * @inheritDoc
   */
  static isValid(value) {
    if (typeof value !== 'number' || (value | 0) !== value) {
      return false;
    }

    return value >= MIN_VALUE && value <= MAX_VALUE;
  }
}

Int.MAX_VALUE = MAX_VALUE;
Int.MIN_VALUE = -MIN_VALUE;
