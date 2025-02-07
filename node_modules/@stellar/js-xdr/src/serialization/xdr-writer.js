const BUFFER_CHUNK = 8192; // 8 KB chunk size increment

/**
 * @internal
 */
export class XdrWriter {
  /**
   * @param {Buffer|Number} [buffer] - Optional destination buffer
   */
  constructor(buffer) {
    if (typeof buffer === 'number') {
      buffer = Buffer.allocUnsafe(buffer);
    } else if (!(buffer instanceof Buffer)) {
      buffer = Buffer.allocUnsafe(BUFFER_CHUNK);
    }
    this._buffer = buffer;
    this._length = buffer.length;
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
  _index = 0;

  /**
   * Advance writer position, write padding if needed, auto-resize the buffer
   * @param {Number} size - Bytes to write
   * @return {Number} Position to read from
   * @private
   */
  alloc(size) {
    const from = this._index;
    // advance cursor position
    this._index += size;
    // ensure sufficient buffer size
    if (this._length < this._index) {
      this.resize(this._index);
    }
    return from;
  }

  /**
   * Increase size of the underlying buffer
   * @param {Number} minRequiredSize - Minimum required buffer size
   * @return {void}
   * @private
   */
  resize(minRequiredSize) {
    // calculate new length, align new buffer length by chunk size
    const newLength = Math.ceil(minRequiredSize / BUFFER_CHUNK) * BUFFER_CHUNK;
    // create new buffer and copy previous data
    const newBuffer = Buffer.allocUnsafe(newLength);
    this._buffer.copy(newBuffer, 0, 0, this._length);
    // update references
    this._buffer = newBuffer;
    this._length = newLength;
  }

  /**
   * Return XDR-serialized value
   * @return {Buffer}
   */
  finalize() {
    // clip underlying buffer to the actually written value
    return this._buffer.subarray(0, this._index);
  }

  /**
   * Return XDR-serialized value as byte array
   * @return {Number[]}
   */
  toArray() {
    return [...this.finalize()];
  }

  /**
   * Write byte array from the buffer
   * @param {Buffer|String} value - Bytes/string to write
   * @param {Number} size - Size in bytes
   * @return {XdrReader} - XdrReader wrapper on top of a subarray
   */
  write(value, size) {
    if (typeof value === 'string') {
      // serialize string directly to the output buffer
      const offset = this.alloc(size);
      this._buffer.write(value, offset, 'utf8');
    } else {
      // copy data to the output buffer
      if (!(value instanceof Buffer)) {
        value = Buffer.from(value);
      }
      const offset = this.alloc(size);
      value.copy(this._buffer, offset, 0, size);
    }

    // add padding for 4-byte XDR alignment
    const padding = 4 - (size % 4 || 4);
    if (padding > 0) {
      const offset = this.alloc(padding);
      this._buffer.fill(0, offset, this._index);
    }
  }

  /**
   * Write i32 from buffer
   * @param {Number} value - Value to serialize
   * @return {void}
   */
  writeInt32BE(value) {
    const offset = this.alloc(4);
    this._buffer.writeInt32BE(value, offset);
  }

  /**
   * Write u32 from buffer
   * @param {Number} value - Value to serialize
   * @return {void}
   */
  writeUInt32BE(value) {
    const offset = this.alloc(4);
    this._buffer.writeUInt32BE(value, offset);
  }

  /**
   * Write i64 from buffer
   * @param {BigInt} value - Value to serialize
   * @return {void}
   */
  writeBigInt64BE(value) {
    const offset = this.alloc(8);
    this._buffer.writeBigInt64BE(value, offset);
  }

  /**
   * Write u64 from buffer
   * @param {BigInt} value - Value to serialize
   * @return {void}
   */
  writeBigUInt64BE(value) {
    const offset = this.alloc(8);
    this._buffer.writeBigUInt64BE(value, offset);
  }

  /**
   * Write float from buffer
   * @param {Number} value - Value to serialize
   * @return {void}
   */
  writeFloatBE(value) {
    const offset = this.alloc(4);
    this._buffer.writeFloatBE(value, offset);
  }

  /**
   * Write double from buffer
   * @param {Number} value - Value to serialize
   * @return {void}
   */
  writeDoubleBE(value) {
    const offset = this.alloc(8);
    this._buffer.writeDoubleBE(value, offset);
  }

  static bufferChunkSize = BUFFER_CHUNK;
}
