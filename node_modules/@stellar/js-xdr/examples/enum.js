import * as XDR from '../src';

let xdr = XDR.config((xdr) => {
  xdr.enum('Color', {
    red: 0,
    green: 1,
    blue: 2
  });

  xdr.enum('ResultType', {
    ok: 0,
    error: 1
  });
});

console.log(xdr);

//
console.log(xdr.Color.members()); // { red: 0, green: 1, blue: 2, }

console.log(xdr.Color.fromName('red'));

console.log(xdr.Color.fromXDR(Buffer.from([0, 0, 0, 0]))); // Color.red
console.log(xdr.Color.red().toXDR()); // Buffer
console.log(xdr.Color.red().toXDR('hex')); //

console.log(xdr.Color.red() !== xdr.ResultType.ok());
