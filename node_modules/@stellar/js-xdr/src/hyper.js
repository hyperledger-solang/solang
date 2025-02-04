import { LargeInt } from './large-int';

export class Hyper extends LargeInt {
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
