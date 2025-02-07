import { XdrReader } from '../../src/serialization/xdr-reader';
import { XdrWriter } from '../../src/serialization/xdr-writer';
let UnsignedInt = XDR.UnsignedInt;

describe('UnsignedInt.read', function () {
  it('decodes correctly', function () {
    expect(read([0x00, 0x00, 0x00, 0x00])).to.eql(0);
    expect(read([0x00, 0x00, 0x00, 0x01])).to.eql(1);
    expect(read([0xff, 0xff, 0xff, 0xff])).to.eql(Math.pow(2, 32) - 1);
  });

  function read(bytes) {
    let io = new XdrReader(bytes);
    return UnsignedInt.read(io);
  }
});

describe('UnsignedInt.write', function () {
  it('encodes correctly', function () {
    expect(write(0)).to.eql([0x00, 0x00, 0x00, 0x00]);
    expect(write(1)).to.eql([0x00, 0x00, 0x00, 0x01]);
    expect(write(Math.pow(2, 32) - 1)).to.eql([0xff, 0xff, 0xff, 0xff]);
  });

  it('throws a write error if the value is not an integral number', function () {
    expect(() => write(true)).to.throw(/write error/i);
    expect(() => write(undefined)).to.throw(/write error/i);
    expect(() => write([])).to.throw(/write error/i);
    expect(() => write({})).to.throw(/write error/i);
    expect(() => write(1.1)).to.throw(/write error/i);
  });

  function write(value) {
    let io = new XdrWriter(8);
    UnsignedInt.write(value, io);
    return io.toArray();
  }
});

describe('UnsignedInt.isValid', function () {
  it('returns true for number in a 32-bit range', function () {
    expect(UnsignedInt.isValid(0)).to.be.true;
    expect(UnsignedInt.isValid(1)).to.be.true;
    expect(UnsignedInt.isValid(1.0)).to.be.true;
    expect(UnsignedInt.isValid(Math.pow(2, 32) - 1)).to.be.true;
  });

  it('returns false for numbers outside a 32-bit range', function () {
    expect(UnsignedInt.isValid(Math.pow(2, 32))).to.be.false;
    expect(UnsignedInt.isValid(-1)).to.be.false;
  });

  it('returns false for non numbers', function () {
    expect(UnsignedInt.isValid(true)).to.be.false;
    expect(UnsignedInt.isValid(false)).to.be.false;
    expect(UnsignedInt.isValid(null)).to.be.false;
    expect(UnsignedInt.isValid('0')).to.be.false;
    expect(UnsignedInt.isValid([])).to.be.false;
    expect(UnsignedInt.isValid([0])).to.be.false;
    expect(UnsignedInt.isValid('hello')).to.be.false;
    expect(UnsignedInt.isValid({ why: 'hello' })).to.be.false;
    expect(UnsignedInt.isValid(['how', 'do', 'you', 'do'])).to.be.false;
    expect(UnsignedInt.isValid(NaN)).to.be.false;
  });

  it('returns false for non-integral values', function () {
    expect(UnsignedInt.isValid(1.1)).to.be.false;
    expect(UnsignedInt.isValid(0.1)).to.be.false;
  });
});
