import * as XDR from '../src';

let xdr = XDR.config((xdr) => {
  xdr.struct('IntList', [
    ['value', xdr.int()],
    ['rest', xdr.option(xdr.lookup('IntList'))]
  ]);
});

let n1 = new xdr.IntList({ value: 1 });
let n2 = new xdr.IntList({ value: 3, rest: n1 });

console.log(n2.toXDR());
