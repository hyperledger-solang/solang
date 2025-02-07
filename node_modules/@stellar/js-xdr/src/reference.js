import { XdrPrimitiveType } from './xdr-type';
import { XdrDefinitionError } from './errors';

export class Reference extends XdrPrimitiveType {
  /* jshint unused: false */
  resolve() {
    throw new XdrDefinitionError(
      '"resolve" method should be implemented in the descendant class'
    );
  }
}
