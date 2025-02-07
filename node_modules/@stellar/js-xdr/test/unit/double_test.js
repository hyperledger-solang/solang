import { XdrWriter } from '../../src/serialization/xdr-writer';
import { XdrReader } from '../../src/serialization/xdr-reader';
let Double = XDR.Double;

describe('Double.read', function () {
  it('decodes correctly', function () {
    expect(read([0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])).to.eql(0.0);
    expect(read([0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])).to.eql(-0.0);
    expect(read([0x3f, 0xf0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])).to.eql(1.0);
    expect(read([0xbf, 0xf0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])).to.eql(-1.0);
    expect(read([0x7f, 0xf8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])).to.eql(NaN);
    expect(read([0x7f, 0xf0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01])).to.eql(NaN);
  });

  function read(bytes) {
    let io = new XdrReader(bytes);
    return Double.read(io);
  }
});

describe('Double.write', function () {
  it('encodes correctly', function () {
    expect(write(0.0)).to.eql([0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    expect(write(-0.0)).to.eql([
      0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00
    ]);
    expect(write(1.0)).to.eql([0x3f, 0xf0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    expect(write(-1.0)).to.eql([
      0xbf, 0xf0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00
    ]);
  });

  function write(value) {
    let io = new XdrWriter(8);
    Double.write(value, io);
    return io.toArray();
  }
});

describe('Double.isValid', function () {
  it('returns true for numbers', function () {
    expect(Double.isValid(0)).to.be.true;
    expect(Double.isValid(-1)).to.be.true;
    expect(Double.isValid(1.0)).to.be.true;
    expect(Double.isValid(100000.0)).to.be.true;
    expect(Double.isValid(NaN)).to.be.true;
    expect(Double.isValid(Infinity)).to.be.true;
    expect(Double.isValid(-Infinity)).to.be.true;
  });

  it('returns false for non numbers', function () {
    expect(Double.isValid(true)).to.be.false;
    expect(Double.isValid(false)).to.be.false;
    expect(Double.isValid(null)).to.be.false;
    expect(Double.isValid('0')).to.be.false;
    expect(Double.isValid([])).to.be.false;
    expect(Double.isValid([0])).to.be.false;
    expect(Double.isValid('hello')).to.be.false;
    expect(Double.isValid({ why: 'hello' })).to.be.false;
    expect(Double.isValid(['how', 'do', 'you', 'do'])).to.be.false;
  });
});
