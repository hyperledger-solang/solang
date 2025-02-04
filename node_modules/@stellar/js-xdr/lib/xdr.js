(function webpackUniversalModuleDefinition(root, factory) {
	if(typeof exports === 'object' && typeof module === 'object')
		module.exports = factory();
	else if(typeof define === 'function' && define.amd)
		define([], factory);
	else if(typeof exports === 'object')
		exports["XDR"] = factory();
	else
		root["XDR"] = factory();
})(this, () => {
return /******/ (() => { // webpackBootstrap
/******/ 	var __webpack_modules__ = ({

/***/ "./src/array.js":
/*!**********************!*\
  !*** ./src/array.js ***!
  \**********************/
/***/ ((__unused_webpack_module, __webpack_exports__, __webpack_require__) => {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   Array: () => (/* binding */ Array)
/* harmony export */ });
/* harmony import */ var _xdr_type__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./xdr-type */ "./src/xdr-type.js");
/* harmony import */ var _errors__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(/*! ./errors */ "./src/errors.js");


class Array extends _xdr_type__WEBPACK_IMPORTED_MODULE_0__.XdrCompositeType {
  constructor(childType, length) {
    super();
    this._childType = childType;
    this._length = length;
  }

  /**
   * @inheritDoc
   */
  read(reader) {
    // allocate array of specified length
    const result = new global.Array(this._length);
    // read values
    for (let i = 0; i < this._length; i++) {
      result[i] = this._childType.read(reader);
    }
    return result;
  }

  /**
   * @inheritDoc
   */
  write(value, writer) {
    if (!global.Array.isArray(value)) throw new _errors__WEBPACK_IMPORTED_MODULE_1__.XdrWriterError(`value is not array`);
    if (value.length !== this._length) throw new _errors__WEBPACK_IMPORTED_MODULE_1__.XdrWriterError(`got array of size ${value.length}, expected ${this._length}`);
    for (const child of value) {
      this._childType.write(child, writer);
    }
  }

  /**
   * @inheritDoc
   */
  isValid(value) {
    if (!(value instanceof global.Array) || value.length !== this._length) {
      return false;
    }
    for (const child of value) {
      if (!this._childType.isValid(child)) return false;
    }
    return true;
  }
}

/***/ }),

/***/ "./src/bigint-encoder.js":
/*!*******************************!*\
  !*** ./src/bigint-encoder.js ***!
  \*******************************/
/***/ ((__unused_webpack_module, __webpack_exports__, __webpack_require__) => {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   calculateBigIntBoundaries: () => (/* binding */ calculateBigIntBoundaries),
/* harmony export */   encodeBigIntFromBits: () => (/* binding */ encodeBigIntFromBits),
/* harmony export */   formatIntName: () => (/* binding */ formatIntName),
/* harmony export */   sliceBigInt: () => (/* binding */ sliceBigInt)
/* harmony export */ });
/**
 * Encode a native `bigint` value from a list of arbitrary integer-like values.
 *
 * @param {Array<number|bigint|string>} parts - Slices to encode in big-endian
 *    format (i.e. earlier elements are higher bits)
 * @param {64|128|256} size - Number of bits in the target integer type
 * @param {boolean} unsigned - Whether it's an unsigned integer
 *
 * @returns {bigint}
 */
function encodeBigIntFromBits(parts, size, unsigned) {
  if (!(parts instanceof Array)) {
    // allow a single parameter instead of an array
    parts = [parts];
  } else if (parts.length && parts[0] instanceof Array) {
    // unpack nested array param
    parts = parts[0];
  }
  const total = parts.length;
  const sliceSize = size / total;
  switch (sliceSize) {
    case 32:
    case 64:
    case 128:
    case 256:
      break;
    default:
      throw new RangeError(`expected slices to fit in 32/64/128/256 bits, got ${parts}`);
  }

  // normalize all inputs to bigint
  try {
    for (let i = 0; i < parts.length; i++) {
      if (typeof parts[i] !== 'bigint') {
        parts[i] = BigInt(parts[i].valueOf());
      }
    }
  } catch (e) {
    throw new TypeError(`expected bigint-like values, got: ${parts} (${e})`);
  }

  // check for sign mismatches for single inputs (this is a special case to
  // handle one parameter passed to e.g. UnsignedHyper et al.)
  // see https://github.com/stellar/js-xdr/pull/100#discussion_r1228770845
  if (unsigned && parts.length === 1 && parts[0] < 0n) {
    throw new RangeError(`expected a positive value, got: ${parts}`);
  }

  // encode in big-endian fashion, shifting each slice by the slice size
  let result = BigInt.asUintN(sliceSize, parts[0]); // safe: len >= 1
  for (let i = 1; i < parts.length; i++) {
    result |= BigInt.asUintN(sliceSize, parts[i]) << BigInt(i * sliceSize);
  }

  // interpret value as signed if necessary and clamp it
  if (!unsigned) {
    result = BigInt.asIntN(size, result);
  }

  // check boundaries
  const [min, max] = calculateBigIntBoundaries(size, unsigned);
  if (result >= min && result <= max) {
    return result;
  }

  // failed to encode
  throw new TypeError(`bigint values [${parts}] for ${formatIntName(size, unsigned)} out of range [${min}, ${max}]: ${result}`);
}

/**
 * Transforms a single bigint value that's supposed to represent a `size`-bit
 * integer into a list of `sliceSize`d chunks.
 *
 * @param {bigint} value - Single bigint value to decompose
 * @param {64|128|256} iSize - Number of bits represented by `value`
 * @param {32|64|128} sliceSize - Number of chunks to decompose into
 * @return {bigint[]}
 */
function sliceBigInt(value, iSize, sliceSize) {
  if (typeof value !== 'bigint') {
    throw new TypeError(`Expected bigint 'value', got ${typeof value}`);
  }
  const total = iSize / sliceSize;
  if (total === 1) {
    return [value];
  }
  if (sliceSize < 32 || sliceSize > 128 || total !== 2 && total !== 4 && total !== 8) {
    throw new TypeError(`invalid bigint (${value}) and slice size (${iSize} -> ${sliceSize}) combination`);
  }
  const shift = BigInt(sliceSize);

  // iterate shift and mask application
  const result = new Array(total);
  for (let i = 0; i < total; i++) {
    // we force a signed interpretation to preserve sign in each slice value,
    // but downstream can convert to unsigned if it's appropriate
    result[i] = BigInt.asIntN(sliceSize, value); // clamps to size

    // move on to the next chunk
    value >>= shift;
  }
  return result;
}
function formatIntName(precision, unsigned) {
  return `${unsigned ? 'u' : 'i'}${precision}`;
}

/**
 * Get min|max boundaries for an integer with a specified bits size
 * @param {64|128|256} size - Number of bits in the source integer type
 * @param {Boolean} unsigned - Whether it's an unsigned integer
 * @return {BigInt[]}
 */
function calculateBigIntBoundaries(size, unsigned) {
  if (unsigned) {
    return [0n, (1n << BigInt(size)) - 1n];
  }
  const boundary = 1n << BigInt(size - 1);
  return [0n - boundary, boundary - 1n];
}

/***/ }),

/***/ "./src/bool.js":
/*!*********************!*\
  !*** ./src/bool.js ***!
  \*********************/
/***/ ((__unused_webpack_module, __webpack_exports__, __webpack_require__) => {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   Bool: () => (/* binding */ Bool)
/* harmony export */ });
/* harmony import */ var _int__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./int */ "./src/int.js");
/* harmony import */ var _xdr_type__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(/*! ./xdr-type */ "./src/xdr-type.js");
/* harmony import */ var _errors__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(/*! ./errors */ "./src/errors.js");



class Bool extends _xdr_type__WEBPACK_IMPORTED_MODULE_1__.XdrPrimitiveType {
  /**
   * @inheritDoc
   */
  static read(reader) {
    const value = _int__WEBPACK_IMPORTED_MODULE_0__.Int.read(reader);
    switch (value) {
      case 0:
        return false;
      case 1:
        return true;
      default:
        throw new _errors__WEBPACK_IMPORTED_MODULE_2__.XdrReaderError(`got ${value} when trying to read a bool`);
    }
  }

  /**
   * @inheritDoc
   */
  static write(value, writer) {
    const intVal = value ? 1 : 0;
    _int__WEBPACK_IMPORTED_MODULE_0__.Int.write(intVal, writer);
  }

  /**
   * @inheritDoc
   */
  static isValid(value) {
    return typeof value === 'boolean';
  }
}

/***/ }),

/***/ "./src/browser.js":
/*!************************!*\
  !*** ./src/browser.js ***!
  \************************/
/***/ ((module, __unused_webpack_exports, __webpack_require__) => {

// eslint-disable-next-line prefer-import/prefer-import-over-require
const exports = __webpack_require__(/*! ./index */ "./src/index.js");
module.exports = exports;

/***/ }),

/***/ "./src/config.js":
/*!***********************!*\
  !*** ./src/config.js ***!
  \***********************/
/***/ ((__unused_webpack_module, __webpack_exports__, __webpack_require__) => {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   Reference: () => (/* reexport safe */ _reference__WEBPACK_IMPORTED_MODULE_1__.Reference),
/* harmony export */   config: () => (/* binding */ config)
/* harmony export */ });
/* harmony import */ var _types__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./types */ "./src/types.js");
/* harmony import */ var _reference__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(/*! ./reference */ "./src/reference.js");
/* harmony import */ var _errors__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(/*! ./errors */ "./src/errors.js");
// eslint-disable-next-line max-classes-per-file




class SimpleReference extends _reference__WEBPACK_IMPORTED_MODULE_1__.Reference {
  constructor(name) {
    super();
    this.name = name;
  }
  resolve(context) {
    const defn = context.definitions[this.name];
    return defn.resolve(context);
  }
}
class ArrayReference extends _reference__WEBPACK_IMPORTED_MODULE_1__.Reference {
  constructor(childReference, length, variable = false) {
    super();
    this.childReference = childReference;
    this.length = length;
    this.variable = variable;
  }
  resolve(context) {
    let resolvedChild = this.childReference;
    let length = this.length;
    if (resolvedChild instanceof _reference__WEBPACK_IMPORTED_MODULE_1__.Reference) {
      resolvedChild = resolvedChild.resolve(context);
    }
    if (length instanceof _reference__WEBPACK_IMPORTED_MODULE_1__.Reference) {
      length = length.resolve(context);
    }
    if (this.variable) {
      return new _types__WEBPACK_IMPORTED_MODULE_0__.VarArray(resolvedChild, length);
    }
    return new _types__WEBPACK_IMPORTED_MODULE_0__.Array(resolvedChild, length);
  }
}
class OptionReference extends _reference__WEBPACK_IMPORTED_MODULE_1__.Reference {
  constructor(childReference) {
    super();
    this.childReference = childReference;
    this.name = childReference.name;
  }
  resolve(context) {
    let resolvedChild = this.childReference;
    if (resolvedChild instanceof _reference__WEBPACK_IMPORTED_MODULE_1__.Reference) {
      resolvedChild = resolvedChild.resolve(context);
    }
    return new _types__WEBPACK_IMPORTED_MODULE_0__.Option(resolvedChild);
  }
}
class SizedReference extends _reference__WEBPACK_IMPORTED_MODULE_1__.Reference {
  constructor(sizedType, length) {
    super();
    this.sizedType = sizedType;
    this.length = length;
  }
  resolve(context) {
    let length = this.length;
    if (length instanceof _reference__WEBPACK_IMPORTED_MODULE_1__.Reference) {
      length = length.resolve(context);
    }
    return new this.sizedType(length);
  }
}
class Definition {
  constructor(constructor, name, cfg) {
    this.constructor = constructor;
    this.name = name;
    this.config = cfg;
  }

  // resolve calls the constructor of this definition with the provided context
  // and this definitions config values.  The definitions constructor should
  // populate the final type on `context.results`, and may refer to other
  // definitions through `context.definitions`
  resolve(context) {
    if (this.name in context.results) {
      return context.results[this.name];
    }
    return this.constructor(context, this.name, this.config);
  }
}

// let the reference resolution system do its thing
// the "constructor" for a typedef just returns the resolved value
function createTypedef(context, typeName, value) {
  if (value instanceof _reference__WEBPACK_IMPORTED_MODULE_1__.Reference) {
    value = value.resolve(context);
  }
  context.results[typeName] = value;
  return value;
}
function createConst(context, name, value) {
  context.results[name] = value;
  return value;
}
class TypeBuilder {
  constructor(destination) {
    this._destination = destination;
    this._definitions = {};
  }
  enum(name, members) {
    const result = new Definition(_types__WEBPACK_IMPORTED_MODULE_0__.Enum.create, name, members);
    this.define(name, result);
  }
  struct(name, members) {
    const result = new Definition(_types__WEBPACK_IMPORTED_MODULE_0__.Struct.create, name, members);
    this.define(name, result);
  }
  union(name, cfg) {
    const result = new Definition(_types__WEBPACK_IMPORTED_MODULE_0__.Union.create, name, cfg);
    this.define(name, result);
  }
  typedef(name, cfg) {
    const result = new Definition(createTypedef, name, cfg);
    this.define(name, result);
  }
  const(name, cfg) {
    const result = new Definition(createConst, name, cfg);
    this.define(name, result);
  }
  void() {
    return _types__WEBPACK_IMPORTED_MODULE_0__.Void;
  }
  bool() {
    return _types__WEBPACK_IMPORTED_MODULE_0__.Bool;
  }
  int() {
    return _types__WEBPACK_IMPORTED_MODULE_0__.Int;
  }
  hyper() {
    return _types__WEBPACK_IMPORTED_MODULE_0__.Hyper;
  }
  uint() {
    return _types__WEBPACK_IMPORTED_MODULE_0__.UnsignedInt;
  }
  uhyper() {
    return _types__WEBPACK_IMPORTED_MODULE_0__.UnsignedHyper;
  }
  float() {
    return _types__WEBPACK_IMPORTED_MODULE_0__.Float;
  }
  double() {
    return _types__WEBPACK_IMPORTED_MODULE_0__.Double;
  }
  quadruple() {
    return _types__WEBPACK_IMPORTED_MODULE_0__.Quadruple;
  }
  string(length) {
    return new SizedReference(_types__WEBPACK_IMPORTED_MODULE_0__.String, length);
  }
  opaque(length) {
    return new SizedReference(_types__WEBPACK_IMPORTED_MODULE_0__.Opaque, length);
  }
  varOpaque(length) {
    return new SizedReference(_types__WEBPACK_IMPORTED_MODULE_0__.VarOpaque, length);
  }
  array(childType, length) {
    return new ArrayReference(childType, length);
  }
  varArray(childType, maxLength) {
    return new ArrayReference(childType, maxLength, true);
  }
  option(childType) {
    return new OptionReference(childType);
  }
  define(name, definition) {
    if (this._destination[name] === undefined) {
      this._definitions[name] = definition;
    } else {
      throw new _errors__WEBPACK_IMPORTED_MODULE_2__.XdrDefinitionError(`${name} is already defined`);
    }
  }
  lookup(name) {
    return new SimpleReference(name);
  }
  resolve() {
    for (const defn of Object.values(this._definitions)) {
      defn.resolve({
        definitions: this._definitions,
        results: this._destination
      });
    }
  }
}
function config(fn, types = {}) {
  if (fn) {
    const builder = new TypeBuilder(types);
    fn(builder);
    builder.resolve();
  }
  return types;
}

/***/ }),

/***/ "./src/double.js":
/*!***********************!*\
  !*** ./src/double.js ***!
  \***********************/
/***/ ((__unused_webpack_module, __webpack_exports__, __webpack_require__) => {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   Double: () => (/* binding */ Double)
/* harmony export */ });
/* harmony import */ var _xdr_type__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./xdr-type */ "./src/xdr-type.js");
/* harmony import */ var _errors__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(/*! ./errors */ "./src/errors.js");


class Double extends _xdr_type__WEBPACK_IMPORTED_MODULE_0__.XdrPrimitiveType {
  /**
   * @inheritDoc
   */
  static read(reader) {
    return reader.readDoubleBE();
  }

  /**
   * @inheritDoc
   */
  static write(value, writer) {
    if (typeof value !== 'number') throw new _errors__WEBPACK_IMPORTED_MODULE_1__.XdrWriterError('not a number');
    writer.writeDoubleBE(value);
  }

  /**
   * @inheritDoc
   */
  static isValid(value) {
    return typeof value === 'number';
  }
}

/***/ }),

/***/ "./src/enum.js":
/*!*********************!*\
  !*** ./src/enum.js ***!
  \*********************/
/***/ ((__unused_webpack_module, __webpack_exports__, __webpack_require__) => {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   Enum: () => (/* binding */ Enum)
/* harmony export */ });
/* harmony import */ var _int__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./int */ "./src/int.js");
/* harmony import */ var _xdr_type__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(/*! ./xdr-type */ "./src/xdr-type.js");
/* harmony import */ var _errors__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(/*! ./errors */ "./src/errors.js");



class Enum extends _xdr_type__WEBPACK_IMPORTED_MODULE_1__.XdrPrimitiveType {
  constructor(name, value) {
    super();
    this.name = name;
    this.value = value;
  }

  /**
   * @inheritDoc
   */
  static read(reader) {
    const intVal = _int__WEBPACK_IMPORTED_MODULE_0__.Int.read(reader);
    const res = this._byValue[intVal];
    if (res === undefined) throw new _errors__WEBPACK_IMPORTED_MODULE_2__.XdrReaderError(`unknown ${this.enumName} member for value ${intVal}`);
    return res;
  }

  /**
   * @inheritDoc
   */
  static write(value, writer) {
    if (!this.isValid(value)) {
      throw new _errors__WEBPACK_IMPORTED_MODULE_2__.XdrWriterError(`${value} has enum name ${value?.enumName}, not ${this.enumName}: ${JSON.stringify(value)}`);
    }
    _int__WEBPACK_IMPORTED_MODULE_0__.Int.write(value.value, writer);
  }

  /**
   * @inheritDoc
   */
  static isValid(value) {
    return value?.constructor?.enumName === this.enumName || (0,_xdr_type__WEBPACK_IMPORTED_MODULE_1__.isSerializableIsh)(value, this);
  }
  static members() {
    return this._members;
  }
  static values() {
    return Object.values(this._members);
  }
  static fromName(name) {
    const result = this._members[name];
    if (!result) throw new TypeError(`${name} is not a member of ${this.enumName}`);
    return result;
  }
  static fromValue(value) {
    const result = this._byValue[value];
    if (result === undefined) throw new TypeError(`${value} is not a value of any member of ${this.enumName}`);
    return result;
  }
  static create(context, name, members) {
    const ChildEnum = class extends Enum {};
    ChildEnum.enumName = name;
    context.results[name] = ChildEnum;
    ChildEnum._members = {};
    ChildEnum._byValue = {};
    for (const [key, value] of Object.entries(members)) {
      const inst = new ChildEnum(key, value);
      ChildEnum._members[key] = inst;
      ChildEnum._byValue[value] = inst;
      ChildEnum[key] = () => inst;
    }
    return ChildEnum;
  }
}

/***/ }),

/***/ "./src/errors.js":
/*!***********************!*\
  !*** ./src/errors.js ***!
  \***********************/
/***/ ((__unused_webpack_module, __webpack_exports__, __webpack_require__) => {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   XdrDefinitionError: () => (/* binding */ XdrDefinitionError),
/* harmony export */   XdrNotImplementedDefinitionError: () => (/* binding */ XdrNotImplementedDefinitionError),
/* harmony export */   XdrReaderError: () => (/* binding */ XdrReaderError),
/* harmony export */   XdrWriterError: () => (/* binding */ XdrWriterError)
/* harmony export */ });
class XdrWriterError extends TypeError {
  constructor(message) {
    super(`XDR Write Error: ${message}`);
  }
}
class XdrReaderError extends TypeError {
  constructor(message) {
    super(`XDR Read Error: ${message}`);
  }
}
class XdrDefinitionError extends TypeError {
  constructor(message) {
    super(`XDR Type Definition Error: ${message}`);
  }
}
class XdrNotImplementedDefinitionError extends XdrDefinitionError {
  constructor() {
    super(`method not implemented, it should be overloaded in the descendant class.`);
  }
}

/***/ }),

/***/ "./src/float.js":
/*!**********************!*\
  !*** ./src/float.js ***!
  \**********************/
/***/ ((__unused_webpack_module, __webpack_exports__, __webpack_require__) => {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   Float: () => (/* binding */ Float)
/* harmony export */ });
/* harmony import */ var _xdr_type__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./xdr-type */ "./src/xdr-type.js");
/* harmony import */ var _errors__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(/*! ./errors */ "./src/errors.js");


class Float extends _xdr_type__WEBPACK_IMPORTED_MODULE_0__.XdrPrimitiveType {
  /**
   * @inheritDoc
   */
  static read(reader) {
    return reader.readFloatBE();
  }

  /**
   * @inheritDoc
   */
  static write(value, writer) {
    if (typeof value !== 'number') throw new _errors__WEBPACK_IMPORTED_MODULE_1__.XdrWriterError('not a number');
    writer.writeFloatBE(value);
  }

  /**
   * @inheritDoc
   */
  static isValid(value) {
    return typeof value === 'number';
  }
}

/***/ }),

/***/ "./src/hyper.js":
/*!**********************!*\
  !*** ./src/hyper.js ***!
  \**********************/
/***/ ((__unused_webpack_module, __webpack_exports__, __webpack_require__) => {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   Hyper: () => (/* binding */ Hyper)
/* harmony export */ });
/* harmony import */ var _large_int__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./large-int */ "./src/large-int.js");

class Hyper extends _large_int__WEBPACK_IMPORTED_MODULE_0__.LargeInt {
  /**
   * @param {Array<Number|BigInt|String>} parts - Slices to encode
   */
  constructor(...args) {
    super(args);
  }
  get low() {
    return Number(this._value & 0xffffffffn) << 0;
  }
  get high() {
    return Number(this._value >> 32n) >> 0;
  }
  get size() {
    return 64;
  }
  get unsigned() {
    return false;
  }

  /**
   * Create Hyper instance from two [high][low] i32 values
   * @param {Number} low - Low part of i64 number
   * @param {Number} high - High part of i64 number
   * @return {LargeInt}
   */
  static fromBits(low, high) {
    return new this(low, high);
  }
}
Hyper.defineIntBoundaries();

/***/ }),

/***/ "./src/index.js":
/*!**********************!*\
  !*** ./src/index.js ***!
  \**********************/
/***/ ((__unused_webpack_module, __webpack_exports__, __webpack_require__) => {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   Array: () => (/* reexport safe */ _types__WEBPACK_IMPORTED_MODULE_0__.Array),
/* harmony export */   Bool: () => (/* reexport safe */ _types__WEBPACK_IMPORTED_MODULE_0__.Bool),
/* harmony export */   Double: () => (/* reexport safe */ _types__WEBPACK_IMPORTED_MODULE_0__.Double),
/* harmony export */   Enum: () => (/* reexport safe */ _types__WEBPACK_IMPORTED_MODULE_0__.Enum),
/* harmony export */   Float: () => (/* reexport safe */ _types__WEBPACK_IMPORTED_MODULE_0__.Float),
/* harmony export */   Hyper: () => (/* reexport safe */ _types__WEBPACK_IMPORTED_MODULE_0__.Hyper),
/* harmony export */   Int: () => (/* reexport safe */ _types__WEBPACK_IMPORTED_MODULE_0__.Int),
/* harmony export */   LargeInt: () => (/* reexport safe */ _types__WEBPACK_IMPORTED_MODULE_0__.LargeInt),
/* harmony export */   Opaque: () => (/* reexport safe */ _types__WEBPACK_IMPORTED_MODULE_0__.Opaque),
/* harmony export */   Option: () => (/* reexport safe */ _types__WEBPACK_IMPORTED_MODULE_0__.Option),
/* harmony export */   Quadruple: () => (/* reexport safe */ _types__WEBPACK_IMPORTED_MODULE_0__.Quadruple),
/* harmony export */   Reference: () => (/* reexport safe */ _config__WEBPACK_IMPORTED_MODULE_1__.Reference),
/* harmony export */   String: () => (/* reexport safe */ _types__WEBPACK_IMPORTED_MODULE_0__.String),
/* harmony export */   Struct: () => (/* reexport safe */ _types__WEBPACK_IMPORTED_MODULE_0__.Struct),
/* harmony export */   Union: () => (/* reexport safe */ _types__WEBPACK_IMPORTED_MODULE_0__.Union),
/* harmony export */   UnsignedHyper: () => (/* reexport safe */ _types__WEBPACK_IMPORTED_MODULE_0__.UnsignedHyper),
/* harmony export */   UnsignedInt: () => (/* reexport safe */ _types__WEBPACK_IMPORTED_MODULE_0__.UnsignedInt),
/* harmony export */   VarArray: () => (/* reexport safe */ _types__WEBPACK_IMPORTED_MODULE_0__.VarArray),
/* harmony export */   VarOpaque: () => (/* reexport safe */ _types__WEBPACK_IMPORTED_MODULE_0__.VarOpaque),
/* harmony export */   Void: () => (/* reexport safe */ _types__WEBPACK_IMPORTED_MODULE_0__.Void),
/* harmony export */   XdrReader: () => (/* reexport safe */ _serialization_xdr_reader__WEBPACK_IMPORTED_MODULE_2__.XdrReader),
/* harmony export */   XdrWriter: () => (/* reexport safe */ _serialization_xdr_writer__WEBPACK_IMPORTED_MODULE_3__.XdrWriter),
/* harmony export */   config: () => (/* reexport safe */ _config__WEBPACK_IMPORTED_MODULE_1__.config)
/* harmony export */ });
/* harmony import */ var _types__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./types */ "./src/types.js");
/* harmony import */ var _config__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(/*! ./config */ "./src/config.js");
/* harmony import */ var _serialization_xdr_reader__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(/*! ./serialization/xdr-reader */ "./src/serialization/xdr-reader.js");
/* harmony import */ var _serialization_xdr_writer__WEBPACK_IMPORTED_MODULE_3__ = __webpack_require__(/*! ./serialization/xdr-writer */ "./src/serialization/xdr-writer.js");





/***/ }),

/***/ "./src/int.js":
/*!********************!*\
  !*** ./src/int.js ***!
  \********************/
/***/ ((__unused_webpack_module, __webpack_exports__, __webpack_require__) => {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   Int: () => (/* binding */ Int)
/* harmony export */ });
/* harmony import */ var _xdr_type__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./xdr-type */ "./src/xdr-type.js");
/* harmony import */ var _errors__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(/*! ./errors */ "./src/errors.js");


const MAX_VALUE = 2147483647;
const MIN_VALUE = -2147483648;
class Int extends _xdr_type__WEBPACK_IMPORTED_MODULE_0__.XdrPrimitiveType {
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
    if (typeof value !== 'number') throw new _errors__WEBPACK_IMPORTED_MODULE_1__.XdrWriterError('not a number');
    if ((value | 0) !== value) throw new _errors__WEBPACK_IMPORTED_MODULE_1__.XdrWriterError('invalid i32 value');
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

/***/ }),

/***/ "./src/large-int.js":
/*!**************************!*\
  !*** ./src/large-int.js ***!
  \**************************/
/***/ ((__unused_webpack_module, __webpack_exports__, __webpack_require__) => {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   LargeInt: () => (/* binding */ LargeInt)
/* harmony export */ });
/* harmony import */ var _xdr_type__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./xdr-type */ "./src/xdr-type.js");
/* harmony import */ var _bigint_encoder__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(/*! ./bigint-encoder */ "./src/bigint-encoder.js");
/* harmony import */ var _errors__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(/*! ./errors */ "./src/errors.js");



class LargeInt extends _xdr_type__WEBPACK_IMPORTED_MODULE_0__.XdrPrimitiveType {
  /**
   * @param {Array<Number|BigInt|String>} parts - Slices to encode
   */
  constructor(args) {
    super();
    this._value = (0,_bigint_encoder__WEBPACK_IMPORTED_MODULE_1__.encodeBigIntFromBits)(args, this.size, this.unsigned);
  }

  /**
   * Signed/unsigned representation
   * @type {Boolean}
   * @abstract
   */
  get unsigned() {
    throw new _errors__WEBPACK_IMPORTED_MODULE_2__.XdrNotImplementedDefinitionError();
  }

  /**
   * Size of the integer in bits
   * @type {Number}
   * @abstract
   */
  get size() {
    throw new _errors__WEBPACK_IMPORTED_MODULE_2__.XdrNotImplementedDefinitionError();
  }

  /**
   * Slice integer to parts with smaller bit size
   * @param {32|64|128} sliceSize - Size of each part in bits
   * @return {BigInt[]}
   */
  slice(sliceSize) {
    return (0,_bigint_encoder__WEBPACK_IMPORTED_MODULE_1__.sliceBigInt)(this._value, this.size, sliceSize);
  }
  toString() {
    return this._value.toString();
  }
  toJSON() {
    return {
      _value: this._value.toString()
    };
  }
  toBigInt() {
    return BigInt(this._value);
  }

  /**
   * @inheritDoc
   */
  static read(reader) {
    const {
      size
    } = this.prototype;
    if (size === 64) return new this(reader.readBigUInt64BE());
    return new this(...Array.from({
      length: size / 64
    }, () => reader.readBigUInt64BE()).reverse());
  }

  /**
   * @inheritDoc
   */
  static write(value, writer) {
    if (value instanceof this) {
      value = value._value;
    } else if (typeof value !== 'bigint' || value > this.MAX_VALUE || value < this.MIN_VALUE) throw new _errors__WEBPACK_IMPORTED_MODULE_2__.XdrWriterError(`${value} is not a ${this.name}`);
    const {
      unsigned,
      size
    } = this.prototype;
    if (size === 64) {
      if (unsigned) {
        writer.writeBigUInt64BE(value);
      } else {
        writer.writeBigInt64BE(value);
      }
    } else {
      for (const part of (0,_bigint_encoder__WEBPACK_IMPORTED_MODULE_1__.sliceBigInt)(value, size, 64).reverse()) {
        if (unsigned) {
          writer.writeBigUInt64BE(part);
        } else {
          writer.writeBigInt64BE(part);
        }
      }
    }
  }

  /**
   * @inheritDoc
   */
  static isValid(value) {
    return typeof value === 'bigint' || value instanceof this;
  }

  /**
   * Create instance from string
   * @param {String} string - Numeric representation
   * @return {LargeInt}
   */
  static fromString(string) {
    return new this(string);
  }
  static MAX_VALUE = 0n;
  static MIN_VALUE = 0n;

  /**
   * @internal
   * @return {void}
   */
  static defineIntBoundaries() {
    const [min, max] = (0,_bigint_encoder__WEBPACK_IMPORTED_MODULE_1__.calculateBigIntBoundaries)(this.prototype.size, this.prototype.unsigned);
    this.MIN_VALUE = min;
    this.MAX_VALUE = max;
  }
}

/***/ }),

/***/ "./src/opaque.js":
/*!***********************!*\
  !*** ./src/opaque.js ***!
  \***********************/
/***/ ((__unused_webpack_module, __webpack_exports__, __webpack_require__) => {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   Opaque: () => (/* binding */ Opaque)
/* harmony export */ });
/* harmony import */ var _xdr_type__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./xdr-type */ "./src/xdr-type.js");
/* harmony import */ var _errors__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(/*! ./errors */ "./src/errors.js");


class Opaque extends _xdr_type__WEBPACK_IMPORTED_MODULE_0__.XdrCompositeType {
  constructor(length) {
    super();
    this._length = length;
  }

  /**
   * @inheritDoc
   */
  read(reader) {
    return reader.read(this._length);
  }

  /**
   * @inheritDoc
   */
  write(value, writer) {
    const {
      length
    } = value;
    if (length !== this._length) throw new _errors__WEBPACK_IMPORTED_MODULE_1__.XdrWriterError(`got ${value.length} bytes, expected ${this._length}`);
    writer.write(value, length);
  }

  /**
   * @inheritDoc
   */
  isValid(value) {
    return Buffer.isBuffer(value) && value.length === this._length;
  }
}

/***/ }),

/***/ "./src/option.js":
/*!***********************!*\
  !*** ./src/option.js ***!
  \***********************/
/***/ ((__unused_webpack_module, __webpack_exports__, __webpack_require__) => {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   Option: () => (/* binding */ Option)
/* harmony export */ });
/* harmony import */ var _bool__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./bool */ "./src/bool.js");
/* harmony import */ var _xdr_type__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(/*! ./xdr-type */ "./src/xdr-type.js");


class Option extends _xdr_type__WEBPACK_IMPORTED_MODULE_1__.XdrPrimitiveType {
  constructor(childType) {
    super();
    this._childType = childType;
  }

  /**
   * @inheritDoc
   */
  read(reader) {
    if (_bool__WEBPACK_IMPORTED_MODULE_0__.Bool.read(reader)) {
      return this._childType.read(reader);
    }
    return undefined;
  }

  /**
   * @inheritDoc
   */
  write(value, writer) {
    const isPresent = value !== null && value !== undefined;
    _bool__WEBPACK_IMPORTED_MODULE_0__.Bool.write(isPresent, writer);
    if (isPresent) {
      this._childType.write(value, writer);
    }
  }

  /**
   * @inheritDoc
   */
  isValid(value) {
    if (value === null || value === undefined) {
      return true;
    }
    return this._childType.isValid(value);
  }
}

/***/ }),

/***/ "./src/quadruple.js":
/*!**************************!*\
  !*** ./src/quadruple.js ***!
  \**************************/
/***/ ((__unused_webpack_module, __webpack_exports__, __webpack_require__) => {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   Quadruple: () => (/* binding */ Quadruple)
/* harmony export */ });
/* harmony import */ var _xdr_type__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./xdr-type */ "./src/xdr-type.js");
/* harmony import */ var _errors__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(/*! ./errors */ "./src/errors.js");


class Quadruple extends _xdr_type__WEBPACK_IMPORTED_MODULE_0__.XdrPrimitiveType {
  static read() {
    throw new _errors__WEBPACK_IMPORTED_MODULE_1__.XdrDefinitionError('quadruple not supported');
  }
  static write() {
    throw new _errors__WEBPACK_IMPORTED_MODULE_1__.XdrDefinitionError('quadruple not supported');
  }
  static isValid() {
    return false;
  }
}

/***/ }),

/***/ "./src/reference.js":
/*!**************************!*\
  !*** ./src/reference.js ***!
  \**************************/
/***/ ((__unused_webpack_module, __webpack_exports__, __webpack_require__) => {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   Reference: () => (/* binding */ Reference)
/* harmony export */ });
/* harmony import */ var _xdr_type__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./xdr-type */ "./src/xdr-type.js");
/* harmony import */ var _errors__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(/*! ./errors */ "./src/errors.js");


class Reference extends _xdr_type__WEBPACK_IMPORTED_MODULE_0__.XdrPrimitiveType {
  /* jshint unused: false */
  resolve() {
    throw new _errors__WEBPACK_IMPORTED_MODULE_1__.XdrDefinitionError('"resolve" method should be implemented in the descendant class');
  }
}

/***/ }),

/***/ "./src/serialization/xdr-reader.js":
/*!*****************************************!*\
  !*** ./src/serialization/xdr-reader.js ***!
  \*****************************************/
/***/ ((__unused_webpack_module, __webpack_exports__, __webpack_require__) => {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   XdrReader: () => (/* binding */ XdrReader)
/* harmony export */ });
/* harmony import */ var _errors__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ../errors */ "./src/errors.js");
/**
 * @internal
 */

class XdrReader {
  /**
   * @constructor
   * @param {Buffer} source - Buffer containing serialized data
   */
  constructor(source) {
    if (!Buffer.isBuffer(source)) {
      if (source instanceof Array || Array.isArray(source) || ArrayBuffer.isView(source)) {
        source = Buffer.from(source);
      } else {
        throw new _errors__WEBPACK_IMPORTED_MODULE_0__.XdrReaderError(`source invalid: ${source}`);
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
    if (this._length < this._index) throw new _errors__WEBPACK_IMPORTED_MODULE_0__.XdrReaderError('attempt to read outside the boundary of the buffer');
    // check that padding is correct for Opaque and String
    const padding = 4 - (size % 4 || 4);
    if (padding > 0) {
      for (let i = 0; i < padding; i++) if (this._buffer[this._index + i] !== 0)
        // all bytes in the padding should be zeros
        throw new _errors__WEBPACK_IMPORTED_MODULE_0__.XdrReaderError('invalid padding');
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
    if (this._index !== this._length) throw new _errors__WEBPACK_IMPORTED_MODULE_0__.XdrReaderError(`invalid XDR contract typecast - source buffer not entirely consumed`);
  }
}

/***/ }),

/***/ "./src/serialization/xdr-writer.js":
/*!*****************************************!*\
  !*** ./src/serialization/xdr-writer.js ***!
  \*****************************************/
/***/ ((__unused_webpack_module, __webpack_exports__, __webpack_require__) => {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   XdrWriter: () => (/* binding */ XdrWriter)
/* harmony export */ });
const BUFFER_CHUNK = 8192; // 8 KB chunk size increment

/**
 * @internal
 */
class XdrWriter {
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

/***/ }),

/***/ "./src/string.js":
/*!***********************!*\
  !*** ./src/string.js ***!
  \***********************/
/***/ ((__unused_webpack_module, __webpack_exports__, __webpack_require__) => {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   String: () => (/* binding */ String)
/* harmony export */ });
/* harmony import */ var _unsigned_int__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./unsigned-int */ "./src/unsigned-int.js");
/* harmony import */ var _xdr_type__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(/*! ./xdr-type */ "./src/xdr-type.js");
/* harmony import */ var _errors__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(/*! ./errors */ "./src/errors.js");



class String extends _xdr_type__WEBPACK_IMPORTED_MODULE_1__.XdrCompositeType {
  constructor(maxLength = _unsigned_int__WEBPACK_IMPORTED_MODULE_0__.UnsignedInt.MAX_VALUE) {
    super();
    this._maxLength = maxLength;
  }

  /**
   * @inheritDoc
   */
  read(reader) {
    const size = _unsigned_int__WEBPACK_IMPORTED_MODULE_0__.UnsignedInt.read(reader);
    if (size > this._maxLength) throw new _errors__WEBPACK_IMPORTED_MODULE_2__.XdrReaderError(`saw ${size} length String, max allowed is ${this._maxLength}`);
    return reader.read(size);
  }
  readString(reader) {
    return this.read(reader).toString('utf8');
  }

  /**
   * @inheritDoc
   */
  write(value, writer) {
    // calculate string byte size before writing
    const size = typeof value === 'string' ? Buffer.byteLength(value, 'utf8') : value.length;
    if (size > this._maxLength) throw new _errors__WEBPACK_IMPORTED_MODULE_2__.XdrWriterError(`got ${value.length} bytes, max allowed is ${this._maxLength}`);
    // write size info
    _unsigned_int__WEBPACK_IMPORTED_MODULE_0__.UnsignedInt.write(size, writer);
    writer.write(value, size);
  }

  /**
   * @inheritDoc
   */
  isValid(value) {
    if (typeof value === 'string') {
      return Buffer.byteLength(value, 'utf8') <= this._maxLength;
    }
    if (value instanceof Array || Buffer.isBuffer(value)) {
      return value.length <= this._maxLength;
    }
    return false;
  }
}

/***/ }),

/***/ "./src/struct.js":
/*!***********************!*\
  !*** ./src/struct.js ***!
  \***********************/
/***/ ((__unused_webpack_module, __webpack_exports__, __webpack_require__) => {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   Struct: () => (/* binding */ Struct)
/* harmony export */ });
/* harmony import */ var _reference__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./reference */ "./src/reference.js");
/* harmony import */ var _xdr_type__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(/*! ./xdr-type */ "./src/xdr-type.js");
/* harmony import */ var _errors__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(/*! ./errors */ "./src/errors.js");



class Struct extends _xdr_type__WEBPACK_IMPORTED_MODULE_1__.XdrCompositeType {
  constructor(attributes) {
    super();
    this._attributes = attributes || {};
  }

  /**
   * @inheritDoc
   */
  static read(reader) {
    const attributes = {};
    for (const [fieldName, type] of this._fields) {
      attributes[fieldName] = type.read(reader);
    }
    return new this(attributes);
  }

  /**
   * @inheritDoc
   */
  static write(value, writer) {
    if (!this.isValid(value)) {
      throw new _errors__WEBPACK_IMPORTED_MODULE_2__.XdrWriterError(`${value} has struct name ${value?.constructor?.structName}, not ${this.structName}: ${JSON.stringify(value)}`);
    }
    for (const [fieldName, type] of this._fields) {
      const attribute = value._attributes[fieldName];
      type.write(attribute, writer);
    }
  }

  /**
   * @inheritDoc
   */
  static isValid(value) {
    return value?.constructor?.structName === this.structName || (0,_xdr_type__WEBPACK_IMPORTED_MODULE_1__.isSerializableIsh)(value, this);
  }
  static create(context, name, fields) {
    const ChildStruct = class extends Struct {};
    ChildStruct.structName = name;
    context.results[name] = ChildStruct;
    const mappedFields = new Array(fields.length);
    for (let i = 0; i < fields.length; i++) {
      const fieldDescriptor = fields[i];
      const fieldName = fieldDescriptor[0];
      let field = fieldDescriptor[1];
      if (field instanceof _reference__WEBPACK_IMPORTED_MODULE_0__.Reference) {
        field = field.resolve(context);
      }
      mappedFields[i] = [fieldName, field];
      // create accessors
      ChildStruct.prototype[fieldName] = createAccessorMethod(fieldName);
    }
    ChildStruct._fields = mappedFields;
    return ChildStruct;
  }
}
function createAccessorMethod(name) {
  return function readOrWriteAttribute(value) {
    if (value !== undefined) {
      this._attributes[name] = value;
    }
    return this._attributes[name];
  };
}

/***/ }),

/***/ "./src/types.js":
/*!**********************!*\
  !*** ./src/types.js ***!
  \**********************/
/***/ ((__unused_webpack_module, __webpack_exports__, __webpack_require__) => {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   Array: () => (/* reexport safe */ _array__WEBPACK_IMPORTED_MODULE_12__.Array),
/* harmony export */   Bool: () => (/* reexport safe */ _bool__WEBPACK_IMPORTED_MODULE_8__.Bool),
/* harmony export */   Double: () => (/* reexport safe */ _double__WEBPACK_IMPORTED_MODULE_6__.Double),
/* harmony export */   Enum: () => (/* reexport safe */ _enum__WEBPACK_IMPORTED_MODULE_16__.Enum),
/* harmony export */   Float: () => (/* reexport safe */ _float__WEBPACK_IMPORTED_MODULE_5__.Float),
/* harmony export */   Hyper: () => (/* reexport safe */ _hyper__WEBPACK_IMPORTED_MODULE_1__.Hyper),
/* harmony export */   Int: () => (/* reexport safe */ _int__WEBPACK_IMPORTED_MODULE_0__.Int),
/* harmony export */   LargeInt: () => (/* reexport safe */ _large_int__WEBPACK_IMPORTED_MODULE_4__.LargeInt),
/* harmony export */   Opaque: () => (/* reexport safe */ _opaque__WEBPACK_IMPORTED_MODULE_10__.Opaque),
/* harmony export */   Option: () => (/* reexport safe */ _option__WEBPACK_IMPORTED_MODULE_14__.Option),
/* harmony export */   Quadruple: () => (/* reexport safe */ _quadruple__WEBPACK_IMPORTED_MODULE_7__.Quadruple),
/* harmony export */   String: () => (/* reexport safe */ _string__WEBPACK_IMPORTED_MODULE_9__.String),
/* harmony export */   Struct: () => (/* reexport safe */ _struct__WEBPACK_IMPORTED_MODULE_17__.Struct),
/* harmony export */   Union: () => (/* reexport safe */ _union__WEBPACK_IMPORTED_MODULE_18__.Union),
/* harmony export */   UnsignedHyper: () => (/* reexport safe */ _unsigned_hyper__WEBPACK_IMPORTED_MODULE_3__.UnsignedHyper),
/* harmony export */   UnsignedInt: () => (/* reexport safe */ _unsigned_int__WEBPACK_IMPORTED_MODULE_2__.UnsignedInt),
/* harmony export */   VarArray: () => (/* reexport safe */ _var_array__WEBPACK_IMPORTED_MODULE_13__.VarArray),
/* harmony export */   VarOpaque: () => (/* reexport safe */ _var_opaque__WEBPACK_IMPORTED_MODULE_11__.VarOpaque),
/* harmony export */   Void: () => (/* reexport safe */ _void__WEBPACK_IMPORTED_MODULE_15__.Void)
/* harmony export */ });
/* harmony import */ var _int__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./int */ "./src/int.js");
/* harmony import */ var _hyper__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(/*! ./hyper */ "./src/hyper.js");
/* harmony import */ var _unsigned_int__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(/*! ./unsigned-int */ "./src/unsigned-int.js");
/* harmony import */ var _unsigned_hyper__WEBPACK_IMPORTED_MODULE_3__ = __webpack_require__(/*! ./unsigned-hyper */ "./src/unsigned-hyper.js");
/* harmony import */ var _large_int__WEBPACK_IMPORTED_MODULE_4__ = __webpack_require__(/*! ./large-int */ "./src/large-int.js");
/* harmony import */ var _float__WEBPACK_IMPORTED_MODULE_5__ = __webpack_require__(/*! ./float */ "./src/float.js");
/* harmony import */ var _double__WEBPACK_IMPORTED_MODULE_6__ = __webpack_require__(/*! ./double */ "./src/double.js");
/* harmony import */ var _quadruple__WEBPACK_IMPORTED_MODULE_7__ = __webpack_require__(/*! ./quadruple */ "./src/quadruple.js");
/* harmony import */ var _bool__WEBPACK_IMPORTED_MODULE_8__ = __webpack_require__(/*! ./bool */ "./src/bool.js");
/* harmony import */ var _string__WEBPACK_IMPORTED_MODULE_9__ = __webpack_require__(/*! ./string */ "./src/string.js");
/* harmony import */ var _opaque__WEBPACK_IMPORTED_MODULE_10__ = __webpack_require__(/*! ./opaque */ "./src/opaque.js");
/* harmony import */ var _var_opaque__WEBPACK_IMPORTED_MODULE_11__ = __webpack_require__(/*! ./var-opaque */ "./src/var-opaque.js");
/* harmony import */ var _array__WEBPACK_IMPORTED_MODULE_12__ = __webpack_require__(/*! ./array */ "./src/array.js");
/* harmony import */ var _var_array__WEBPACK_IMPORTED_MODULE_13__ = __webpack_require__(/*! ./var-array */ "./src/var-array.js");
/* harmony import */ var _option__WEBPACK_IMPORTED_MODULE_14__ = __webpack_require__(/*! ./option */ "./src/option.js");
/* harmony import */ var _void__WEBPACK_IMPORTED_MODULE_15__ = __webpack_require__(/*! ./void */ "./src/void.js");
/* harmony import */ var _enum__WEBPACK_IMPORTED_MODULE_16__ = __webpack_require__(/*! ./enum */ "./src/enum.js");
/* harmony import */ var _struct__WEBPACK_IMPORTED_MODULE_17__ = __webpack_require__(/*! ./struct */ "./src/struct.js");
/* harmony import */ var _union__WEBPACK_IMPORTED_MODULE_18__ = __webpack_require__(/*! ./union */ "./src/union.js");




















/***/ }),

/***/ "./src/union.js":
/*!**********************!*\
  !*** ./src/union.js ***!
  \**********************/
/***/ ((__unused_webpack_module, __webpack_exports__, __webpack_require__) => {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   Union: () => (/* binding */ Union)
/* harmony export */ });
/* harmony import */ var _void__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./void */ "./src/void.js");
/* harmony import */ var _reference__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(/*! ./reference */ "./src/reference.js");
/* harmony import */ var _xdr_type__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(/*! ./xdr-type */ "./src/xdr-type.js");
/* harmony import */ var _errors__WEBPACK_IMPORTED_MODULE_3__ = __webpack_require__(/*! ./errors */ "./src/errors.js");




class Union extends _xdr_type__WEBPACK_IMPORTED_MODULE_2__.XdrCompositeType {
  constructor(aSwitch, value) {
    super();
    this.set(aSwitch, value);
  }
  set(aSwitch, value) {
    if (typeof aSwitch === 'string') {
      aSwitch = this.constructor._switchOn.fromName(aSwitch);
    }
    this._switch = aSwitch;
    const arm = this.constructor.armForSwitch(this._switch);
    this._arm = arm;
    this._armType = arm === _void__WEBPACK_IMPORTED_MODULE_0__.Void ? _void__WEBPACK_IMPORTED_MODULE_0__.Void : this.constructor._arms[arm];
    this._value = value;
  }
  get(armName = this._arm) {
    if (this._arm !== _void__WEBPACK_IMPORTED_MODULE_0__.Void && this._arm !== armName) throw new TypeError(`${armName} not set`);
    return this._value;
  }
  switch() {
    return this._switch;
  }
  arm() {
    return this._arm;
  }
  armType() {
    return this._armType;
  }
  value() {
    return this._value;
  }
  static armForSwitch(aSwitch) {
    const member = this._switches.get(aSwitch);
    if (member !== undefined) {
      return member;
    }
    if (this._defaultArm) {
      return this._defaultArm;
    }
    throw new TypeError(`Bad union switch: ${aSwitch}`);
  }
  static armTypeForArm(arm) {
    if (arm === _void__WEBPACK_IMPORTED_MODULE_0__.Void) {
      return _void__WEBPACK_IMPORTED_MODULE_0__.Void;
    }
    return this._arms[arm];
  }

  /**
   * @inheritDoc
   */
  static read(reader) {
    const aSwitch = this._switchOn.read(reader);
    const arm = this.armForSwitch(aSwitch);
    const armType = arm === _void__WEBPACK_IMPORTED_MODULE_0__.Void ? _void__WEBPACK_IMPORTED_MODULE_0__.Void : this._arms[arm];
    let value;
    if (armType !== undefined) {
      value = armType.read(reader);
    } else {
      value = arm.read(reader);
    }
    return new this(aSwitch, value);
  }

  /**
   * @inheritDoc
   */
  static write(value, writer) {
    if (!this.isValid(value)) {
      throw new _errors__WEBPACK_IMPORTED_MODULE_3__.XdrWriterError(`${value} has union name ${value?.unionName}, not ${this.unionName}: ${JSON.stringify(value)}`);
    }
    this._switchOn.write(value.switch(), writer);
    value.armType().write(value.value(), writer);
  }

  /**
   * @inheritDoc
   */
  static isValid(value) {
    return value?.constructor?.unionName === this.unionName || (0,_xdr_type__WEBPACK_IMPORTED_MODULE_2__.isSerializableIsh)(value, this);
  }
  static create(context, name, config) {
    const ChildUnion = class extends Union {};
    ChildUnion.unionName = name;
    context.results[name] = ChildUnion;
    if (config.switchOn instanceof _reference__WEBPACK_IMPORTED_MODULE_1__.Reference) {
      ChildUnion._switchOn = config.switchOn.resolve(context);
    } else {
      ChildUnion._switchOn = config.switchOn;
    }
    ChildUnion._switches = new Map();
    ChildUnion._arms = {};

    // resolve default arm
    let defaultArm = config.defaultArm;
    if (defaultArm instanceof _reference__WEBPACK_IMPORTED_MODULE_1__.Reference) {
      defaultArm = defaultArm.resolve(context);
    }
    ChildUnion._defaultArm = defaultArm;
    for (const [aSwitch, armName] of config.switches) {
      const key = typeof aSwitch === 'string' ? ChildUnion._switchOn.fromName(aSwitch) : aSwitch;
      ChildUnion._switches.set(key, armName);
    }

    // add enum-based helpers
    // NOTE: we don't have good notation for "is a subclass of XDR.Enum",
    //  and so we use the following check (does _switchOn have a `values`
    //  attribute) to approximate the intent.
    if (ChildUnion._switchOn.values !== undefined) {
      for (const aSwitch of ChildUnion._switchOn.values()) {
        // Add enum-based constructors
        ChildUnion[aSwitch.name] = function ctr(value) {
          return new ChildUnion(aSwitch, value);
        };

        // Add enum-based "set" helpers
        ChildUnion.prototype[aSwitch.name] = function set(value) {
          return this.set(aSwitch, value);
        };
      }
    }
    if (config.arms) {
      for (const [armsName, value] of Object.entries(config.arms)) {
        ChildUnion._arms[armsName] = value instanceof _reference__WEBPACK_IMPORTED_MODULE_1__.Reference ? value.resolve(context) : value;
        // Add arm accessor helpers
        if (value !== _void__WEBPACK_IMPORTED_MODULE_0__.Void) {
          ChildUnion.prototype[armsName] = function get() {
            return this.get(armsName);
          };
        }
      }
    }
    return ChildUnion;
  }
}

/***/ }),

/***/ "./src/unsigned-hyper.js":
/*!*******************************!*\
  !*** ./src/unsigned-hyper.js ***!
  \*******************************/
/***/ ((__unused_webpack_module, __webpack_exports__, __webpack_require__) => {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   UnsignedHyper: () => (/* binding */ UnsignedHyper)
/* harmony export */ });
/* harmony import */ var _large_int__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./large-int */ "./src/large-int.js");

class UnsignedHyper extends _large_int__WEBPACK_IMPORTED_MODULE_0__.LargeInt {
  /**
   * @param {Array<Number|BigInt|String>} parts - Slices to encode
   */
  constructor(...args) {
    super(args);
  }
  get low() {
    return Number(this._value & 0xffffffffn) << 0;
  }
  get high() {
    return Number(this._value >> 32n) >> 0;
  }
  get size() {
    return 64;
  }
  get unsigned() {
    return true;
  }

  /**
   * Create UnsignedHyper instance from two [high][low] i32 values
   * @param {Number} low - Low part of u64 number
   * @param {Number} high - High part of u64 number
   * @return {UnsignedHyper}
   */
  static fromBits(low, high) {
    return new this(low, high);
  }
}
UnsignedHyper.defineIntBoundaries();

/***/ }),

/***/ "./src/unsigned-int.js":
/*!*****************************!*\
  !*** ./src/unsigned-int.js ***!
  \*****************************/
/***/ ((__unused_webpack_module, __webpack_exports__, __webpack_require__) => {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   UnsignedInt: () => (/* binding */ UnsignedInt)
/* harmony export */ });
/* harmony import */ var _xdr_type__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./xdr-type */ "./src/xdr-type.js");
/* harmony import */ var _errors__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(/*! ./errors */ "./src/errors.js");


const MAX_VALUE = 4294967295;
const MIN_VALUE = 0;
class UnsignedInt extends _xdr_type__WEBPACK_IMPORTED_MODULE_0__.XdrPrimitiveType {
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
    if (typeof value !== 'number' || !(value >= MIN_VALUE && value <= MAX_VALUE) || value % 1 !== 0) throw new _errors__WEBPACK_IMPORTED_MODULE_1__.XdrWriterError('invalid u32 value');
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

/***/ }),

/***/ "./src/var-array.js":
/*!**************************!*\
  !*** ./src/var-array.js ***!
  \**************************/
/***/ ((__unused_webpack_module, __webpack_exports__, __webpack_require__) => {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   VarArray: () => (/* binding */ VarArray)
/* harmony export */ });
/* harmony import */ var _unsigned_int__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./unsigned-int */ "./src/unsigned-int.js");
/* harmony import */ var _xdr_type__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(/*! ./xdr-type */ "./src/xdr-type.js");
/* harmony import */ var _errors__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(/*! ./errors */ "./src/errors.js");



class VarArray extends _xdr_type__WEBPACK_IMPORTED_MODULE_1__.XdrCompositeType {
  constructor(childType, maxLength = _unsigned_int__WEBPACK_IMPORTED_MODULE_0__.UnsignedInt.MAX_VALUE) {
    super();
    this._childType = childType;
    this._maxLength = maxLength;
  }

  /**
   * @inheritDoc
   */
  read(reader) {
    const length = _unsigned_int__WEBPACK_IMPORTED_MODULE_0__.UnsignedInt.read(reader);
    if (length > this._maxLength) throw new _errors__WEBPACK_IMPORTED_MODULE_2__.XdrReaderError(`saw ${length} length VarArray, max allowed is ${this._maxLength}`);
    const result = new Array(length);
    for (let i = 0; i < length; i++) {
      result[i] = this._childType.read(reader);
    }
    return result;
  }

  /**
   * @inheritDoc
   */
  write(value, writer) {
    if (!(value instanceof Array)) throw new _errors__WEBPACK_IMPORTED_MODULE_2__.XdrWriterError(`value is not array`);
    if (value.length > this._maxLength) throw new _errors__WEBPACK_IMPORTED_MODULE_2__.XdrWriterError(`got array of size ${value.length}, max allowed is ${this._maxLength}`);
    _unsigned_int__WEBPACK_IMPORTED_MODULE_0__.UnsignedInt.write(value.length, writer);
    for (const child of value) {
      this._childType.write(child, writer);
    }
  }

  /**
   * @inheritDoc
   */
  isValid(value) {
    if (!(value instanceof Array) || value.length > this._maxLength) {
      return false;
    }
    for (const child of value) {
      if (!this._childType.isValid(child)) return false;
    }
    return true;
  }
}

/***/ }),

/***/ "./src/var-opaque.js":
/*!***************************!*\
  !*** ./src/var-opaque.js ***!
  \***************************/
/***/ ((__unused_webpack_module, __webpack_exports__, __webpack_require__) => {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   VarOpaque: () => (/* binding */ VarOpaque)
/* harmony export */ });
/* harmony import */ var _unsigned_int__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./unsigned-int */ "./src/unsigned-int.js");
/* harmony import */ var _xdr_type__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(/*! ./xdr-type */ "./src/xdr-type.js");
/* harmony import */ var _errors__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(/*! ./errors */ "./src/errors.js");



class VarOpaque extends _xdr_type__WEBPACK_IMPORTED_MODULE_1__.XdrCompositeType {
  constructor(maxLength = _unsigned_int__WEBPACK_IMPORTED_MODULE_0__.UnsignedInt.MAX_VALUE) {
    super();
    this._maxLength = maxLength;
  }

  /**
   * @inheritDoc
   */
  read(reader) {
    const size = _unsigned_int__WEBPACK_IMPORTED_MODULE_0__.UnsignedInt.read(reader);
    if (size > this._maxLength) throw new _errors__WEBPACK_IMPORTED_MODULE_2__.XdrReaderError(`saw ${size} length VarOpaque, max allowed is ${this._maxLength}`);
    return reader.read(size);
  }

  /**
   * @inheritDoc
   */
  write(value, writer) {
    const {
      length
    } = value;
    if (value.length > this._maxLength) throw new _errors__WEBPACK_IMPORTED_MODULE_2__.XdrWriterError(`got ${value.length} bytes, max allowed is ${this._maxLength}`);
    // write size info
    _unsigned_int__WEBPACK_IMPORTED_MODULE_0__.UnsignedInt.write(length, writer);
    writer.write(value, length);
  }

  /**
   * @inheritDoc
   */
  isValid(value) {
    return Buffer.isBuffer(value) && value.length <= this._maxLength;
  }
}

/***/ }),

/***/ "./src/void.js":
/*!*********************!*\
  !*** ./src/void.js ***!
  \*********************/
/***/ ((__unused_webpack_module, __webpack_exports__, __webpack_require__) => {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   Void: () => (/* binding */ Void)
/* harmony export */ });
/* harmony import */ var _xdr_type__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./xdr-type */ "./src/xdr-type.js");
/* harmony import */ var _errors__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(/*! ./errors */ "./src/errors.js");


class Void extends _xdr_type__WEBPACK_IMPORTED_MODULE_0__.XdrPrimitiveType {
  /* jshint unused: false */

  static read() {
    return undefined;
  }
  static write(value) {
    if (value !== undefined) throw new _errors__WEBPACK_IMPORTED_MODULE_1__.XdrWriterError('trying to write value to a void slot');
  }
  static isValid(value) {
    return value === undefined;
  }
}

/***/ }),

/***/ "./src/xdr-type.js":
/*!*************************!*\
  !*** ./src/xdr-type.js ***!
  \*************************/
/***/ ((__unused_webpack_module, __webpack_exports__, __webpack_require__) => {

"use strict";
__webpack_require__.r(__webpack_exports__);
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   XdrCompositeType: () => (/* binding */ XdrCompositeType),
/* harmony export */   XdrPrimitiveType: () => (/* binding */ XdrPrimitiveType),
/* harmony export */   hasConstructor: () => (/* binding */ hasConstructor),
/* harmony export */   isSerializableIsh: () => (/* binding */ isSerializableIsh)
/* harmony export */ });
/* harmony import */ var _serialization_xdr_reader__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./serialization/xdr-reader */ "./src/serialization/xdr-reader.js");
/* harmony import */ var _serialization_xdr_writer__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(/*! ./serialization/xdr-writer */ "./src/serialization/xdr-writer.js");
/* harmony import */ var _errors__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(/*! ./errors */ "./src/errors.js");



class XdrType {
  /**
   * Encode value to XDR format
   * @param {XdrEncodingFormat} [format] - Encoding format (one of "raw", "hex", "base64")
   * @return {String|Buffer}
   */
  toXDR(format = 'raw') {
    if (!this.write) return this.constructor.toXDR(this, format);
    const writer = new _serialization_xdr_writer__WEBPACK_IMPORTED_MODULE_1__.XdrWriter();
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
    const reader = new _serialization_xdr_reader__WEBPACK_IMPORTED_MODULE_0__.XdrReader(decodeInput(input, format));
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
    const writer = new _serialization_xdr_writer__WEBPACK_IMPORTED_MODULE_1__.XdrWriter();
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
    const reader = new _serialization_xdr_reader__WEBPACK_IMPORTED_MODULE_0__.XdrReader(decodeInput(input, format));
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
class XdrPrimitiveType extends XdrType {
  /**
   * Read value from the XDR-serialized input
   * @param {XdrReader} reader - XdrReader instance
   * @return {this}
   * @abstract
   */
  // eslint-disable-next-line no-unused-vars
  static read(reader) {
    throw new _errors__WEBPACK_IMPORTED_MODULE_2__.XdrNotImplementedDefinitionError();
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
    throw new _errors__WEBPACK_IMPORTED_MODULE_2__.XdrNotImplementedDefinitionError();
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
class XdrCompositeType extends XdrType {
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
function isSerializableIsh(value, subtype) {
  return value !== undefined && value !== null && (
  // prereqs, otherwise `getPrototypeOf` pops
  value instanceof subtype ||
  // quickest check
  // Do an initial constructor check (anywhere is fine so that children of
  // `subtype` still work), then
  hasConstructor(value, subtype) &&
  // ensure it has read/write methods, then
  typeof value.constructor.read === 'function' && typeof value.constructor.write === 'function' &&
  // ensure XdrType is in the prototype chain
  hasConstructor(value, 'XdrType'));
}

/** Tries to find `subtype` in any of the constructors or meta of `instance`. */
function hasConstructor(instance, subtype) {
  do {
    const ctor = instance.constructor;
    if (ctor.name === subtype) {
      return true;
    }
  } while (instance = Object.getPrototypeOf(instance));
  return false;
}

/**
 * @typedef {'raw'|'hex'|'base64'} XdrEncodingFormat
 */

/***/ })

/******/ 	});
/************************************************************************/
/******/ 	// The module cache
/******/ 	var __webpack_module_cache__ = {};
/******/ 	
/******/ 	// The require function
/******/ 	function __webpack_require__(moduleId) {
/******/ 		// Check if module is in cache
/******/ 		var cachedModule = __webpack_module_cache__[moduleId];
/******/ 		if (cachedModule !== undefined) {
/******/ 			return cachedModule.exports;
/******/ 		}
/******/ 		// Create a new module (and put it into the cache)
/******/ 		var module = __webpack_module_cache__[moduleId] = {
/******/ 			// no module.id needed
/******/ 			// no module.loaded needed
/******/ 			exports: {}
/******/ 		};
/******/ 	
/******/ 		// Execute the module function
/******/ 		__webpack_modules__[moduleId](module, module.exports, __webpack_require__);
/******/ 	
/******/ 		// Return the exports of the module
/******/ 		return module.exports;
/******/ 	}
/******/ 	
/************************************************************************/
/******/ 	/* webpack/runtime/define property getters */
/******/ 	(() => {
/******/ 		// define getter functions for harmony exports
/******/ 		__webpack_require__.d = (exports, definition) => {
/******/ 			for(var key in definition) {
/******/ 				if(__webpack_require__.o(definition, key) && !__webpack_require__.o(exports, key)) {
/******/ 					Object.defineProperty(exports, key, { enumerable: true, get: definition[key] });
/******/ 				}
/******/ 			}
/******/ 		};
/******/ 	})();
/******/ 	
/******/ 	/* webpack/runtime/hasOwnProperty shorthand */
/******/ 	(() => {
/******/ 		__webpack_require__.o = (obj, prop) => (Object.prototype.hasOwnProperty.call(obj, prop))
/******/ 	})();
/******/ 	
/******/ 	/* webpack/runtime/make namespace object */
/******/ 	(() => {
/******/ 		// define __esModule on exports
/******/ 		__webpack_require__.r = (exports) => {
/******/ 			if(typeof Symbol !== 'undefined' && Symbol.toStringTag) {
/******/ 				Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });
/******/ 			}
/******/ 			Object.defineProperty(exports, '__esModule', { value: true });
/******/ 		};
/******/ 	})();
/******/ 	
/************************************************************************/
/******/ 	
/******/ 	// startup
/******/ 	// Load entry module and return exports
/******/ 	// This entry module is referenced by other modules so it can't be inlined
/******/ 	var __webpack_exports__ = __webpack_require__("./src/browser.js");
/******/ 	
/******/ 	return __webpack_exports__;
/******/ })()
;
});
//# sourceMappingURL=xdr.js.map