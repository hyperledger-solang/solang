import { XdrReader } from '../../src/serialization/xdr-reader';

/* jshint -W030 */

let emptyContext = { definitions: {}, results: {} };

let Ext = XDR.Union.create(emptyContext, 'Ext', {
  switchOn: XDR.Int,
  switches: [
    [0, XDR.Void],
    [1, XDR.Int]
  ]
});

let StructUnion = XDR.Struct.create(emptyContext, 'StructUnion', [
  ['id', XDR.Int],
  ['ext', Ext]
]);

describe('StructUnion.read', function () {
  it('decodes correctly', function () {
    let empty = read([0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00]);
    expect(empty).to.be.instanceof(StructUnion);
    expect(empty.id()).to.eql(1);
    expect(empty.ext().switch()).to.eql(0);
    expect(empty.ext().arm()).to.eql(XDR.Void);

    let filled = read([
      0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02
    ]);

    expect(filled).to.be.instanceof(StructUnion);
    expect(filled.id()).to.eql(2);
    expect(filled.ext().switch()).to.eql(1);
    expect(filled.ext().arm()).to.eql(XDR.Int);
    expect(filled.ext().value()).to.eql(2);
  });

  function read(bytes) {
    let io = new XdrReader(bytes);
    return StructUnion.read(io);
  }
});
