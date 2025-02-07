import { XdrPrimitiveType } from './xdr-type';
import { XdrWriterError } from './errors';

const MAX_VALUE = 4294967295;
const MIN_VALUE = 0;

export class UnsignedInt extends XdrPrimitiveType {
  /**
   * @inheritDoc
   */
  static read(reader) {
    return reader.readUInt32BE();
  }

  /**
   * @inheritDoc
   */
  static write(value, writer) {
    if (
      typeof value !== 'number' ||
      !(value >= MIN_VALUE && value <= MAX_VALUE) ||
      value % 1 !== 0
    )
      throw new XdrWriterError('invalid u32 value');

    writer.writeUInt32BE(value);
  }

  /**
   * @inheritDoc
   */
  static isValid(value) {
    if (typeof value !== 'number' || value % 1 !== 0) {
      return false;
    }

    return value >= MIN_VALUE && value <= MAX_VALUE;
  }
}

UnsignedInt.MAX_VALUE = MAX_VALUE;
UnsignedInt.MIN_VALUE = MIN_VALUE;
