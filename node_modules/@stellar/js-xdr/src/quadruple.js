import { XdrPrimitiveType } from './xdr-type';
import { XdrDefinitionError } from './errors';

export class Quadruple extends XdrPrimitiveType {
  static read() {
    throw new XdrDefinitionError('quadruple not supported');
  }

  static write() {
    throw new XdrDefinitionError('quadruple not supported');
  }

  static isValid() {
    return false;
  }
}
