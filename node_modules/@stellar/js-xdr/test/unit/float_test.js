let Float = XDR.Float;
import { XdrReader } from '../../src/serialization/xdr-reader';
import { XdrWriter } from '../../src/serialization/xdr-writer';

describe('Float.read', function () {
  it('decodes correctly', function () {
    expect(read([0x00, 0x00, 0x00, 0x00])).to.eql(0.0);
    expect(read([0x80, 0x00, 0x00, 0x00])).to.eql(-0.0);
    expect(read([0x3f, 0x80, 0x00, 0x00])).to.eql(1.0);
    expect(read([0xbf, 0x80, 0x00, 0x00])).to.eql(-1.0);
    expect(read([0x7f, 0xc0, 0x00, 0x00])).to.eql(NaN);
    expect(read([0x7f, 0xf8, 0x00, 0x00])).to.eql(NaN);
  });

  function read(bytes) {
    let io = new XdrReader(bytes);
    return Float.read(io);
  }
});

describe('Float.write', function () {
  it('encodes correctly', function () {
    expect(write(0.0)).to.eql([0x00, 0x00, 0x00, 0x00]);
    expect(write(-0.0)).to.eql([0x80, 0x00, 0x00, 0x00]);
    expect(write(1.0)).to.eql([0x3f, 0x80, 0x00, 0x00]);
    expect(write(-1.0)).to.eql([0xbf, 0x80, 0x00, 0x00]);
  });

  function write(value) {
    let io = new XdrWriter(8);
    Float.write(value, io);
    return io.toArray();
  }
});

describe('Float.isValid', function () {
  it('returns true for numbers', function () {
    expect(Float.isValid(0)).to.be.true;
    expect(Float.isValid(-1)).to.be.true;
    expect(Float.isValid(1.0)).to.be.true;
    expect(Float.isValid(100000.0)).to.be.true;
    expect(Float.isValid(NaN)).to.be.true;
    expect(Float.isValid(Infinity)).to.be.true;
    expect(Float.isValid(-Infinity)).to.be.true;
  });

  it('returns false for non numbers', function () {
    expect(Float.isValid(true)).to.be.false;
    expect(Float.isValid(false)).to.be.false;
    expect(Float.isValid(null)).to.be.false;
    expect(Float.isValid('0')).to.be.false;
    expect(Float.isValid([])).to.be.false;
    expect(Float.isValid([0])).to.be.false;
    expect(Float.isValid('hello')).to.be.false;
    expect(Float.isValid({ why: 'hello' })).to.be.false;
    expect(Float.isValid(['how', 'do', 'you', 'do'])).to.be.false;
  });
});
