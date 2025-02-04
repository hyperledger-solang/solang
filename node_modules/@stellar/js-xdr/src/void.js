import { XdrPrimitiveType } from './xdr-type';
import { XdrWriterError } from './errors';

export class Void extends XdrPrimitiveType {
  /* jshint unused: false */

  static read() {
    return undefined;
  }

  static write(value) {
    if (value !== undefined)
      throw new XdrWriterError('trying to write value to a void slot');
  }

  static isValid(value) {
    return value === undefined;
  }
}
