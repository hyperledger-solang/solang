import { XdrWriter } from '../../src/serialization/xdr-writer';
import { XdrReader } from '../../src/serialization/xdr-reader';
let Bool = XDR.Bool;

describe('Bool.read', function () {
  it('decodes correctly', function () {
    expect(read([0, 0, 0, 0])).to.eql(false);
    expect(read([0, 0, 0, 1])).to.eql(true);

    expect(() => read([0, 0, 0, 2])).to.throw(/read error/i);
    expect(() => read([255, 255, 255, 255])).to.throw(/read error/i);
  });

  function read(bytes) {
    let io = new XdrReader(bytes);
    return Bool.read(io);
  }
});

describe('Bool.write', function () {
  it('encodes correctly', function () {
    expect(write(false)).to.eql([0, 0, 0, 0]);
    expect(write(true)).to.eql([0, 0, 0, 1]);
  });

  function write(value) {
    let io = new XdrWriter(8);
    Bool.write(value, io);
    return io.toArray();
  }
});

describe('Bool.isValid', function () {
  it('returns true for booleans', function () {
    expect(Bool.isValid(true)).to.be.true;
    expect(Bool.isValid(false)).to.be.true;
  });

  it('returns false for non booleans', function () {
    expect(Bool.isValid(0)).to.be.false;
    expect(Bool.isValid('0')).to.be.false;
    expect(Bool.isValid([true])).to.be.false;
    expect(Bool.isValid(null)).to.be.false;
    expect(Bool.isValid({})).to.be.false;
    expect(Bool.isValid(undefined)).to.be.false;
  });
});
