/**
 * @internal
 */
import { XdrReaderError } from '../errors';

export class XdrReader {
  /**
   * @constructor
   * @param {Buffer} source - Buffer containing serialized data
   */
  constructor(source) {
    if (!Buffer.isBuffer(source)) {
      if (
        source instanceof Array ||
        Array.isArray(source) ||
        ArrayBuffer.isView(source)
      ) {
        source = Buffer.from(source);
      } else {
        throw new XdrReaderError(`source invalid: ${source}`);
      }
    }

    this._buffer = source;
    this._length = source.length;
    this._index = 0;
  }

  /**
   * @type {Buffer}
   * @private
   * @readonly
   */
  _buffer;
  /**
   * @type {Number}
   * @private
   * @readonly
   */
  _length;
  /**
   * @type {Number}
   * @private
   * @readonly
   */
  _index;

  /**
   * Check if the reader reached the end of the input buffer
   * @return {Boolean}
   */
  get eof() {
    return this._index === this._length;
  }

  /**
   * Advance reader position, check padding and overflow
   * @param {Number} size - Bytes to read
   * @return {Number} Position to read from
   * @private
   */
  advance(size) {
    const from = this._index;
    // advance cursor position
    this._index += size;
    // check buffer boundaries
    if (this._length < this._index)
      throw new XdrReaderError(
        'attempt to read outside the boundary of the buffer'
      );
    // check that padding is correct for Opaque and String
    const padding = 4 - (size % 4 || 4);
    if (padding > 0) {
      for (let i = 0; i < padding; i++)
        if (this._buffer[this._index + i] !== 0)
          // all bytes in the padding should be zeros
          throw new XdrReaderError('invalid padding');
      this._index += padding;
    }
    return from;
  }

  /**
   * Reset reader position
   * @return {void}
   */
  rewind() {
    this._index = 0;
  }

  /**
   * Read byte array from the buffer
   * @param {Number} size - Bytes to read
   * @return {Buffer} - Sliced portion of the underlying buffer
   */
  read(size) {
    const from = this.advance(size);
    return this._buffer.subarray(from, from + size);
  }

  /**
   * Read i32 from buffer
   * @return {Number}
   */
  readInt32BE() {
    return this._buffer.readInt32BE(this.advance(4));
  }

  /**
   * Read u32 from buffer
   * @return {Number}
   */
  readUInt32BE() {
    return this._buffer.readUInt32BE(this.advance(4));
  }

  /**
   * Read i64 from buffer
   * @return {BigInt}
   */
  readBigInt64BE() {
    return this._buffer.readBigInt64BE(this.advance(8));
  }

  /**
   * Read u64 from buffer
   * @return {BigInt}
   */
  readBigUInt64BE() {
    return this._buffer.readBigUInt64BE(this.advance(8));
  }

  /**
   * Read float from buffer
   * @return {Number}
   */
  readFloatBE() {
    return this._buffer.readFloatBE(this.advance(4));
  }

  /**
   * Read double from buffer
   * @return {Number}
   */
  readDoubleBE() {
    return this._buffer.readDoubleBE(this.advance(8));
  }

  /**
   * Ensure that input buffer has been consumed in full, otherwise it's a type mismatch
   * @return {void}
   * @throws {XdrReaderError}
   */
  ensureInputConsumed() {
    if (this._index !== this._length)
      throw new XdrReaderError(
        `invalid XDR contract typecast - source buffer not entirely consumed`
      );
  }
}
