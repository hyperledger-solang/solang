import { XdrWriter } from '../../src/serialization/xdr-writer';
import { XdrReader } from '../../src/serialization/xdr-reader';

const Quadruple = XDR.Quadruple;

describe('Quadruple.read', function () {
  it('is not supported', function () {
    expect(() => read([0x00, 0x00, 0x00, 0x00])).to.throw(
      /Type Definition Error/i
    );
  });

  function read(bytes) {
    let io = new XdrReader(bytes);
    return Quadruple.read(io);
  }
});

describe('Quadruple.write', function () {
  it('is not supported', function () {
    expect(() => write(0.0)).to.throw(/Type Definition Error/i);
  });

  function write(value) {
    let io = new XdrWriter(8);
    Quadruple.write(value, io);
    return io.toArray();
  }
});

describe('Quadruple.isValid', function () {
  it('returns false', function () {
    expect(Quadruple.isValid(1.0)).to.be.false;
  });
});
