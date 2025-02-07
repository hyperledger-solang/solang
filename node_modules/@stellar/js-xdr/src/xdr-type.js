import { XdrReader } from './serialization/xdr-reader';
import { XdrWriter } from './serialization/xdr-writer';
import { XdrNotImplementedDefinitionError } from './errors';

class XdrType {
  /**
   * Encode value to XDR format
   * @param {XdrEncodingFormat} [format] - Encoding format (one of "raw", "hex", "base64")
   * @return {String|Buffer}
   */
  toXDR(format = 'raw') {
    if (!this.write) return this.constructor.toXDR(this, format);

    const writer = new XdrWriter();
    this.write(this, writer);
    return encodeResult(writer.finalize(), format);
  }

  /**
   * Decode XDR-encoded value
   * @param {Buffer|String} input - XDR-encoded input data
   * @param {XdrEncodingFormat} [format] - Encoding format (one of "raw", "hex", "base64")
   * @return {this}
   */
  fromXDR(input, format = 'raw') {
    if (!this.read) return this.constructor.fromXDR(input, format);

    const reader = new XdrReader(decodeInput(input, format));
    const result = this.read(reader);
    reader.ensureInputConsumed();
    return result;
  }

  /**
   * Check whether input contains a valid XDR-encoded value
   * @param {Buffer|String} input - XDR-encoded input data
   * @param {XdrEncodingFormat} [format] - Encoding format (one of "raw", "hex", "base64")
   * @return {Boolean}
   */
  validateXDR(input, format = 'raw') {
    try {
      this.fromXDR(input, format);
      return true;
    } catch (e) {
      return false;
    }
  }

  /**
   * Encode value to XDR format
   * @param {this} value - Value to serialize
   * @param {XdrEncodingFormat} [format] - Encoding format (one of "raw", "hex", "base64")
   * @return {Buffer}
   */
  static toXDR(value, format = 'raw') {
    const writer = new XdrWriter();
    this.write(value, writer);
    return encodeResult(writer.finalize(), format);
  }

  /**
   * Decode XDR-encoded value
   * @param {Buffer|String} input - XDR-encoded input data
   * @param {XdrEncodingFormat} [format] - Encoding format (one of "raw", "hex", "base64")
   * @return {this}
   */
  static fromXDR(input, format = 'raw') {
    const reader = new XdrReader(decodeInput(input, format));
    const result = this.read(reader);
    reader.ensureInputConsumed();
    return result;
  }

  /**
   * Check whether input contains a valid XDR-encoded value
   * @param {Buffer|String} input - XDR-encoded input data
   * @param {XdrEncodingFormat} [format] - Encoding format (one of "raw", "hex", "base64")
   * @return {Boolean}
   */
  static validateXDR(input, format = 'raw') {
    try {
      this.fromXDR(input, format);
      return true;
    } catch (e) {
      return false;
    }
  }
}

export class XdrPrimitiveType extends XdrType {
  /**
   * Read value from the XDR-serialized input
   * @param {XdrReader} reader - XdrReader instance
   * @return {this}
   * @abstract
   */
  // eslint-disable-next-line no-unused-vars
  static read(reader) {
    throw new XdrNotImplementedDefinitionError();
  }

  /**
   * Write XDR value to the buffer
   * @param {this} value - Value to write
   * @param {XdrWriter} writer - XdrWriter instance
   * @return {void}
   * @abstract
   */
  // eslint-disable-next-line no-unused-vars
  static write(value, writer) {
    throw new XdrNotImplementedDefinitionError();
  }

  /**
   * Check whether XDR primitive value is valid
   * @param {this} value - Value to check
   * @return {Boolean}
   * @abstract
   */
  // eslint-disable-next-line no-unused-vars
  static isValid(value) {
    return false;
  }
}

export class XdrCompositeType extends XdrType {
  // Every descendant should implement two methods: read(reader) and write(value, writer)

  /**
   * Check whether XDR primitive value is valid
   * @param {this} value - Value to check
   * @return {Boolean}
   * @abstract
   */
  // eslint-disable-next-line no-unused-vars
  isValid(value) {
    return false;
  }
}

class InvalidXdrEncodingFormatError extends TypeError {
  constructor(format) {
    super(`Invalid format ${format}, must be one of "raw", "hex", "base64"`);
  }
}

function encodeResult(buffer, format) {
  switch (format) {
    case 'raw':
      return buffer;
    case 'hex':
      return buffer.toString('hex');
    case 'base64':
      return buffer.toString('base64');
    default:
      throw new InvalidXdrEncodingFormatError(format);
  }
}

function decodeInput(input, format) {
  switch (format) {
    case 'raw':
      return input;
    case 'hex':
      return Buffer.from(input, 'hex');
    case 'base64':
      return Buffer.from(input, 'base64');
    default:
      throw new InvalidXdrEncodingFormatError(format);
  }
}

/**
 * Provides a "duck typed" version of the native `instanceof` for read/write.
 *
 * "Duck typing" means if the parameter _looks like_ and _acts like_ a duck
 * (i.e. the type we're checking), it will be treated as that type.
 *
 * In this case, the "type" we're looking for is "like XdrType" but also "like
 * XdrCompositeType|XdrPrimitiveType" (i.e. serializable), but also conditioned
 * on a particular subclass of "XdrType" (e.g. {@link Union} which extends
 * XdrType).
 *
 * This makes the package resilient to downstream systems that may be combining
 * many versions of a package across its stack that are technically compatible
 * but fail `instanceof` checks due to cross-pollination.
 */
export function isSerializableIsh(value, subtype) {
  return (
    value !== undefined &&
    value !== null && // prereqs, otherwise `getPrototypeOf` pops
    (value instanceof subtype || // quickest check
      // Do an initial constructor check (anywhere is fine so that children of
      // `subtype` still work), then
      (hasConstructor(value, subtype) &&
        // ensure it has read/write methods, then
        typeof value.constructor.read === 'function' &&
        typeof value.constructor.write === 'function' &&
        // ensure XdrType is in the prototype chain
        hasConstructor(value, 'XdrType')))
  );
}

/** Tries to find `subtype` in any of the constructors or meta of `instance`. */
export function hasConstructor(instance, subtype) {
  do {
    const ctor = instance.constructor;
    if (ctor.name === subtype) {
      return true;
    }
  } while ((instance = Object.getPrototypeOf(instance)));
  return false;
}

/**
 * @typedef {'raw'|'hex'|'base64'} XdrEncodingFormat
 */
