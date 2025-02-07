import { XdrReader } from '../../src/serialization/xdr-reader';
import { XdrWriter } from '../../src/serialization/xdr-writer';

let subject = new XDR.String(4);

describe('String#read', function () {
  it('decodes correctly', function () {
    expect(read([0x00, 0x00, 0x00, 0x00]).toString('utf8')).to.eql('');
    expect(
      read([0x00, 0x00, 0x00, 0x01, 0x41, 0x00, 0x00, 0x00]).toString('utf8')
    ).to.eql('A');
    expect(
      read([0x00, 0x00, 0x00, 0x03, 0xe4, 0xb8, 0x89, 0x00]).toString('utf8')
    ).to.eql('三');
    expect(
      read([0x00, 0x00, 0x00, 0x02, 0x41, 0x41, 0x00, 0x00]).toString('utf8')
    ).to.eql('AA');
  });

  it('decodes correctly to string', function () {
    expect(readString([0x00, 0x00, 0x00, 0x00])).to.eql('');
    expect(readString([0x00, 0x00, 0x00, 0x01, 0x41, 0x00, 0x00, 0x00])).to.eql(
      'A'
    );
    expect(readString([0x00, 0x00, 0x00, 0x03, 0xe4, 0xb8, 0x89, 0x00])).to.eql(
      '三'
    );
    expect(readString([0x00, 0x00, 0x00, 0x02, 0x41, 0x41, 0x00, 0x00])).to.eql(
      'AA'
    );
  });

  it('decodes non-utf-8 correctly', function () {
    let val = read([0x00, 0x00, 0x00, 0x01, 0xd1, 0x00, 0x00, 0x00]);
    expect(val[0]).to.eql(0xd1);
  });

  it('throws a read error when the encoded length is greater than the allowed max', function () {
    expect(() =>
      read([0x00, 0x00, 0x00, 0x05, 0x41, 0x41, 0x41, 0x41, 0x41])
    ).to.throw(/read error/i);
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
    return subject.read(io);
  }

  function readString(bytes) {
    const io = new XdrReader(bytes);
    const res = subject.readString(io);
    expect(io._index).to.eql(
      !res ? 4 : 8,
      'padding not processed by the reader'
    );
    return res;
  }
});

describe('String#write', function () {
  it('encodes string correctly', function () {
    expect(write('')).to.eql([0x00, 0x00, 0x00, 0x00]);
    expect(write('三')).to.eql([
      0x00, 0x00, 0x00, 0x03, 0xe4, 0xb8, 0x89, 0x00
    ]);
    expect(write('A')).to.eql([0x00, 0x00, 0x00, 0x01, 0x41, 0x00, 0x00, 0x00]);
    expect(write('AA')).to.eql([
      0x00, 0x00, 0x00, 0x02, 0x41, 0x41, 0x00, 0x00
    ]);
  });

  it('encodes non-utf-8 correctly', function () {
    expect(write([0xd1])).to.eql([
      0x00, 0x00, 0x00, 0x01, 0xd1, 0x00, 0x00, 0x00
    ]);
  });

  it('encodes non-utf-8 correctly (buffer)', function () {
    expect(write(Buffer.from([0xd1]))).to.eql([
      0x00, 0x00, 0x00, 0x01, 0xd1, 0x00, 0x00, 0x00
    ]);
  });

  it('checks actual utf-8 strings length on write', function () {
    expect(() => write('€€€€')).to.throw(/max allowed/i);
  });

  function write(value) {
    let io = new XdrWriter(8);
    subject.write(value, io);
    return io.toArray();
  }
});

describe('String#isValid', function () {
  it('returns true for strings of the correct length', function () {
    expect(subject.isValid('')).to.be.true;
    expect(subject.isValid('a')).to.be.true;
    expect(subject.isValid('aa')).to.be.true;
  });

  it('returns true for arrays of the correct length', function () {
    expect(subject.isValid([0x01])).to.be.true;
  });

  it('returns true for buffers of the correct length', function () {
    expect(subject.isValid(Buffer.from([0x01]))).to.be.true;
  });

  it('returns false for strings that are too large', function () {
    expect(subject.isValid('aaaaa')).to.be.false;
  });

  it('returns false for arrays that are too large', function () {
    expect(subject.isValid([0x01, 0x01, 0x01, 0x01, 0x01])).to.be.false;
  });

  it('returns false for buffers that are too large', function () {
    expect(subject.isValid(Buffer.from([0x01, 0x01, 0x01, 0x01, 0x01]))).to.be
      .false;
  });

  it('returns false for non string/array/buffer', function () {
    expect(subject.isValid(true)).to.be.false;
    expect(subject.isValid(null)).to.be.false;
    expect(subject.isValid(3)).to.be.false;
  });
});

describe('String#constructor', function () {
  let subject = new XDR.String();

  it('defaults to max length of a uint max value', function () {
    expect(subject._maxLength).to.eql(Math.pow(2, 32) - 1);
  });
});
