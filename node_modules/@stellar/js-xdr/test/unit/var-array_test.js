import { XdrReader } from '../../src/serialization/xdr-reader';
import { XdrWriter } from '../../src/serialization/xdr-writer';

const subject = new XDR.VarArray(XDR.Int, 2);

describe('VarArray#read', function () {
  it('decodes correctly', function () {
    expect(read([0x00, 0x00, 0x00, 0x00])).to.eql([]);

    expect(read([0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00])).to.eql([0]);
    expect(read([0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01])).to.eql([1]);

    expect(
      read([
        0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01
      ])
    ).to.eql([0, 1]);
    expect(
      read([
        0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01
      ])
    ).to.eql([1, 1]);
  });

  it('throws read error when the encoded array is too large', function () {
    expect(() => read([0x00, 0x00, 0x00, 0x03])).to.throw(/read error/i);
  });

  function read(bytes) {
    let io = new XdrReader(bytes);
    return subject.read(io);
  }
});

describe('VarArray#write', function () {
  it('encodes correctly', function () {
    expect(write([])).to.eql([0x00, 0x00, 0x00, 0x00]);
    expect(write([0])).to.eql([0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00]);
    expect(write([0, 1])).to.eql([
      0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01
    ]);
  });

  it('throws a write error if the value is too large', function () {
    expect(() => write([1, 2, 3])).to.throw(/write error/i);
  });

  it('throws a write error if a child element is of the wrong type', function () {
    expect(() => write([1, null])).to.throw(/write error/i);
    expect(() => write([1, undefined])).to.throw(/write error/i);
    expect(() => write([1, 'hi'])).to.throw(/write error/i);
  });

  function write(value) {
    let io = new XdrWriter(256);
    subject.write(value, io);
    return io.toArray();
  }
});

describe('VarArray#isValid', function () {
  it('returns true for an array of the correct sizes with the correct types', function () {
    expect(subject.isValid([])).to.be.true;
    expect(subject.isValid([1])).to.be.true;
    expect(subject.isValid([1, 2])).to.be.true;
  });

  it('returns false for arrays of the wrong size', function () {
    expect(subject.isValid([1, 2, 3])).to.be.false;
  });

  it('returns false if a child element is invalid for the child type', function () {
    expect(subject.isValid([1, null])).to.be.false;
    expect(subject.isValid([1, undefined])).to.be.false;
    expect(subject.isValid([1, 'hello'])).to.be.false;
    expect(subject.isValid([1, []])).to.be.false;
    expect(subject.isValid([1, {}])).to.be.false;
  });
});

describe('VarArray#constructor', function () {
  let subject = new XDR.VarArray(XDR.Int);

  it('defaults to max length of a uint max value', function () {
    expect(subject._maxLength).to.eql(Math.pow(2, 32) - 1);
  });
});
