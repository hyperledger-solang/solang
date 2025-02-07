import { XdrReader } from '../../src/serialization/xdr-reader';
import { XdrWriter } from '../../src/serialization/xdr-writer';

const VarOpaque = XDR.VarOpaque;

let subject = new VarOpaque(2);

describe('VarOpaque#read', function () {
  it('decodes correctly', function () {
    expect(read([0, 0, 0, 0])).to.eql(Buffer.from([]));
    expect(read([0, 0, 0, 1, 0, 0, 0, 0])).to.eql(Buffer.from([0]));
    expect(read([0, 0, 0, 1, 1, 0, 0, 0])).to.eql(Buffer.from([1]));
    expect(read([0, 0, 0, 2, 0, 1, 0, 0])).to.eql(Buffer.from([0, 1]));
  });

  it('throws a read error when the encoded length is greater than the allowed max', function () {
    expect(() => read([0, 0, 0, 3, 0, 0, 0, 0])).to.throw(/read error/i);
  });

  it('throws a read error if the padding bytes are not zero', function () {
    expect(() =>
      read([0x00, 0x00, 0x00, 0x01, 0x41, 0x01, 0x00, 0x00])
    ).to.throw(/read error/i);
    expect(() =>
      read([0x00, 0x00, 0x00, 0x01, 0x41, 0x00, 0x01, 0x00])
    ).to.throw(/read error/i);
    expect(() =>
      read([0x00, 0x00, 0x00, 0x01, 0x41, 0x00, 0x00, 0x01])
    ).to.throw(/read error/i);
  });

  function read(bytes) {
    let io = new XdrReader(bytes);
    const res = subject.read(io);
    expect(io._index).to.eql(
      !res.length ? 4 : 8,
      'padding not processed by the reader'
    );
    return res;
  }
});

describe('VarOpaque#write', function () {
  it('encodes correctly', function () {
    expect(write(Buffer.from([]))).to.eql([0, 0, 0, 0]);
    expect(write(Buffer.from([0]))).to.eql([0, 0, 0, 1, 0, 0, 0, 0]);
    expect(write(Buffer.from([1]))).to.eql([0, 0, 0, 1, 1, 0, 0, 0]);
    expect(write(Buffer.from([0, 1]))).to.eql([0, 0, 0, 2, 0, 1, 0, 0]);
  });

  function write(value) {
    let io = new XdrWriter(8);
    subject.write(value, io);
    return io.toArray();
  }
});

describe('VarOpaque#isValid', function () {
  it('returns true for buffers of the correct length', function () {
    expect(subject.isValid(Buffer.alloc(0))).to.be.true;
    expect(subject.isValid(Buffer.alloc(1))).to.be.true;
    expect(subject.isValid(Buffer.alloc(2))).to.be.true;
  });

  it('returns false for buffers of the wrong size', function () {
    expect(subject.isValid(Buffer.alloc(3))).to.be.false;
    expect(subject.isValid(Buffer.alloc(3000))).to.be.false;
  });

  it('returns false for non buffers', function () {
    expect(subject.isValid(true)).to.be.false;
    expect(subject.isValid(null)).to.be.false;
    expect(subject.isValid(3)).to.be.false;
    expect(subject.isValid([0])).to.be.false;
  });
});

describe('VarOpaque#constructor', function () {
  let subject = new XDR.VarOpaque();

  it('defaults to max length of a uint max value', function () {
    expect(subject._maxLength).to.eql(Math.pow(2, 32) - 1);
  });
});
