import { Int } from './int';
import { XdrPrimitiveType } from './xdr-type';
import { XdrReaderError } from './errors';

export class Bool extends XdrPrimitiveType {
  /**
   * @inheritDoc
   */
  static read(reader) {
    const value = Int.read(reader);

    switch (value) {
      case 0:
        return false;
      case 1:
        return true;
      default:
        throw new XdrReaderError(`got ${value} when trying to read a bool`);
    }
  }

  /**
   * @inheritDoc
   */
  static write(value, writer) {
    const intVal = value ? 1 : 0;
    Int.write(intVal, writer);
  }

  /**
   * @inheritDoc
   */
  static isValid(value) {
    return typeof value === 'boolean';
  }
}
