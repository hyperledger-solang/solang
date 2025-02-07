import { XdrWriter } from '../../src/serialization/xdr-writer';
import { XdrReader } from '../../src/serialization/xdr-reader';
let Int = XDR.Int;

describe('Int.read', function () {
  it('decodes correctly', function () {
    expect(read([0x00, 0x00, 0x00, 0x00])).to.eql(0);
    expect(read([0x00, 0x00, 0x00, 0x01])).to.eql(1);
    expect(read([0xff, 0xff, 0xff, 0xff])).to.eql(-1);
    expect(read([0x7f, 0xff, 0xff, 0xff])).to.eql(Math.pow(2, 31) - 1);
    expect(read([0x80, 0x00, 0x00, 0x00])).to.eql(-Math.pow(2, 31));
  });

  function read(bytes) {
    let io = new XdrReader(bytes);
    return Int.read(io);
  }
});

describe('Int.write', function () {
  it('encodes correctly', function () {
    expect(write(0)).to.eql([0x00, 0x00, 0x00, 0x00]);
    expect(write(1)).to.eql([0x00, 0x00, 0x00, 0x01]);
    expect(write(-1)).to.eql([0xff, 0xff, 0xff, 0xff]);
    expect(write(Math.pow(2, 31) - 1)).to.eql([0x7f, 0xff, 0xff, 0xff]);
    expect(write(-Math.pow(2, 31))).to.eql([0x80, 0x00, 0x00, 0x00]);
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
    Int.write(value, io);
    return io.toArray();
  }
});

describe('Int.isValid', function () {
  it('returns true for number in a 32-bit range', function () {
    expect(Int.isValid(0)).to.be.true;
    expect(Int.isValid(-1)).to.be.true;
    expect(Int.isValid(1.0)).to.be.true;
    expect(Int.isValid(Math.pow(2, 31) - 1)).to.be.true;
    expect(Int.isValid(-Math.pow(2, 31))).to.be.true;
  });

  it('returns false for numbers outside a 32-bit range', function () {
    expect(Int.isValid(Math.pow(2, 31))).to.be.false;
    expect(Int.isValid(-(Math.pow(2, 31) + 1))).to.be.false;
    expect(Int.isValid(1000000000000)).to.be.false;
  });

  it('returns false for non numbers', function () {
    expect(Int.isValid(true)).to.be.false;
    expect(Int.isValid(false)).to.be.false;
    expect(Int.isValid(null)).to.be.false;
    expect(Int.isValid('0')).to.be.false;
    expect(Int.isValid([])).to.be.false;
    expect(Int.isValid([0])).to.be.false;
    expect(Int.isValid('hello')).to.be.false;
    expect(Int.isValid({ why: 'hello' })).to.be.false;
    expect(Int.isValid(['how', 'do', 'you', 'do'])).to.be.false;
    expect(Int.isValid(NaN)).to.be.false;
  });

  it('returns false for non-integral values', function () {
    expect(Int.isValid(1.1)).to.be.false;
    expect(Int.isValid(0.1)).to.be.false;
    expect(Int.isValid(-0.1)).to.be.false;
    expect(Int.isValid(-1.1)).to.be.false;
  });
});
