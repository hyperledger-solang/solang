import { XdrWriter } from '../../src/serialization/xdr-writer';

describe('Dynamic writer buffer resize', function () {
  it('automatically resize buffer', function () {
    const str = new XDR.String(32768);
    let io = new XdrWriter(12);
    str.write('7 bytes', io);
    // expect buffer size to equal base size
    expect(io._buffer.length).to.eql(12);
    str.write('a'.repeat(32768), io);
    // expect buffer growth up to 5 chunks
    expect(io._buffer.length).to.eql(40960);
    // increase by 1 more 8 KB chunk
    str.write('a'.repeat(9000), io);
    expect(io._buffer.length).to.eql(49152);
    // check final buffer size
    expect(io.toArray().length).to.eql(41788);
  });
});
