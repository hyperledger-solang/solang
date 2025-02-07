import { XdrReader } from '../../src/serialization/xdr-reader';
import { XdrWriter } from '../../src/serialization/xdr-writer';

let zero = new XDR.Array(XDR.Int, 0);
let one = new XDR.Array(XDR.Int, 1);
let many = new XDR.Array(XDR.Int, 2);

describe('Array#read', function () {
  it('decodes correctly', function () {
    expect(read(zero, [])).to.eql([]);
    expect(read(zero, [0x00, 0x00, 0x00, 0x00])).to.eql([]);

    expect(read(one, [0x00, 0x00, 0x00, 0x00])).to.eql([0]);
    expect(read(one, [0x00, 0x00, 0x00, 0x01])).to.eql([1]);

    expect(read(many, [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01])).to.eql(
      [0, 1]
    );
    expect(read(many, [0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01])).to.eql(
      [1, 1]
    );
  });

  it("throws XdrReaderError when the byte stream isn't large enough", function () {
    expect(() => read(many, [0x00, 0x00, 0x00, 0x00])).to.throw(
      /read outside the boundary/i
    );
  });

  function read(arr, bytes) {
    let reader = new XdrReader(bytes);
    return arr.read(reader);
  }
});

describe('Array#write', function () {
  let subject = many;

  it('encodes correctly', function () {
    expect(write([1, 2])).to.eql([
      0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02
    ]);
    expect(write([3, 4])).to.eql([
      0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x04
    ]);
  });

  it('throws a write error if the value is not the correct length', function () {
    expect(() => write(null)).to.throw(/write error/i);
    expect(() => write(undefined)).to.throw(/write error/i);
    expect(() => write([])).to.throw(/write error/i);
    expect(() => write([1])).to.throw(/write error/i);
    expect(() => write([1, 2, 3])).to.throw(/write error/i);
  });

  it('throws a write error if a child element is of the wrong type', function () {
    expect(() => write([1, null])).to.throw(/write error/i);
    expect(() => write([1, undefined])).to.throw(/write error/i);
    expect(() => write([1, 'hi'])).to.throw(/write error/i);
  });

  function write(value) {
    let writer = new XdrWriter(8);
    subject.write(value, writer);
    return writer.toArray();
  }
});

describe('Array#isValid', function () {
  let subject = many;

  it('returns true for an array of the correct size with the correct types', function () {
    expect(subject.isValid([1, 2])).to.be.true;
  });

  it('returns false for arrays of the wrong size', function () {
    expect(subject.isValid([])).to.be.false;
    expect(subject.isValid([1])).to.be.false;
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
