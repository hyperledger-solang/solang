// See https://github.com/stellar/js-xdr/issues/117
import { Buffer } from 'buffer';

if (!(Buffer.alloc(1).subarray(0, 1) instanceof Buffer)) {
  Buffer.prototype.subarray = function subarray(start, end) {
    const result = Uint8Array.prototype.subarray.call(this, start, end);
    Object.setPrototypeOf(result, Buffer.prototype);
    return result;
  };
}

export default Buffer;
