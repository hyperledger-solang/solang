import { XdrReader } from '../../src/serialization/xdr-reader';
import { XdrWriter } from '../../src/serialization/xdr-writer';

let subject = XDR.Void;

describe('Void#read', function () {
  it('decodes correctly', function () {
    expect(read([0x00, 0x00, 0x00, 0x00])).to.be.undefined;
    expect(read([0x00, 0x00, 0x00, 0x01])).to.be.undefined;
    expect(read([0x00, 0x00, 0x00, 0x02])).to.be.undefined;
  });

  function read(bytes) {
    let io = new XdrReader(bytes);
    return subject.read(io);
  }
});

describe('Void#write', function () {
  it('encodes correctly', function () {
    expect(write(undefined)).to.eql([]);
  });

  function write(value) {
    let io = new XdrWriter(8);
    subject.write(value, io);
    return io.toArray();
  }
});

describe('Void#isValid', function () {
  it('returns true undefined', function () {
    expect(subject.isValid(undefined)).to.be.true;
  });

  it('returns false for anything defined', function () {
    expect(subject.isValid(null)).to.be.false;
    expect(subject.isValid(false)).to.be.false;
    expect(subject.isValid(1)).to.be.false;
    expect(subject.isValid('aaa')).to.be.false;
    expect(subject.isValid({})).to.be.false;
    expect(subject.isValid([undefined])).to.be.false;
  });
});
