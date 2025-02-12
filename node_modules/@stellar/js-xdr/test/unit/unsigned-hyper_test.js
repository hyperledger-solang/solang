import { XdrReader } from '../../src/serialization/xdr-reader';
import { XdrWriter } from '../../src/serialization/xdr-writer';

const UnsignedHyper = XDR.UnsignedHyper;

describe('UnsignedHyper.read', function () {
  it('decodes correctly', function () {
    expect(read([0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])).to.eql(
      UnsignedHyper.fromString('0')
    );
    expect(read([0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01])).to.eql(
      UnsignedHyper.fromString('1')
    );
    expect(read([0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff])).to.eql(
      new UnsignedHyper(UnsignedHyper.MAX_VALUE)
    );
  });

  function read(bytes) {
    let io = new XdrReader(bytes);
    return UnsignedHyper.read(io);
  }
});

describe('UnsignedHyper.write', function () {
  it('encodes correctly', function () {
    expect(write(UnsignedHyper.fromString('0'))).to.eql([
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00
    ]);
    expect(write(UnsignedHyper.fromString('1'))).to.eql([
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01
    ]);
    expect(write(UnsignedHyper.MAX_VALUE)).to.eql([
      0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff
    ]);
  });

  function write(value) {
    let io = new XdrWriter(8);
    UnsignedHyper.write(value, io);
    return io.toArray();
  }
});

describe('UnsignedHyper.isValid', function () {
  it('returns true for UnsignedHyper instances', function () {
    expect(UnsignedHyper.isValid(UnsignedHyper.fromString('1'))).to.be.true;
    expect(UnsignedHyper.isValid(UnsignedHyper.MIN_VALUE)).to.be.true;
    expect(UnsignedHyper.isValid(UnsignedHyper.MAX_VALUE)).to.be.true;
  });

  it('returns false for non UnsignedHypers', function () {
    expect(UnsignedHyper.isValid(null)).to.be.false;
    expect(UnsignedHyper.isValid(undefined)).to.be.false;
    expect(UnsignedHyper.isValid([])).to.be.false;
    expect(UnsignedHyper.isValid({})).to.be.false;
    expect(UnsignedHyper.isValid(1)).to.be.false;
    expect(UnsignedHyper.isValid(true)).to.be.false;
  });
});

describe('UnsignedHyper.fromString', function () {
  it('works for positive numbers', function () {
    expect(UnsignedHyper.fromString('1059').toString()).to.eql('1059');
  });

  it('fails for negative numbers', function () {
    expect(() => UnsignedHyper.fromString('-1059')).to.throw(/positive/);
  });

  it('fails when providing a string with a decimal place', function () {
    expect(() => UnsignedHyper.fromString('105946095601.5')).to.throw(/bigint/);
  });
});
