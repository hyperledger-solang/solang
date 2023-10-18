
contract Foo {
    @account(oneAccount)
    @signer(mySigner)
    @mutableAccount(otherAccount)
    @mutableSigner(otherSigner)
    function bar() external returns (uint64) {
        assert(tx.accounts.mySigner.is_signer);
        assert(tx.accounts.otherSigner.is_signer);
        assert(tx.accounts.otherSigner.is_writable);
        assert(tx.accounts.otherAccount.is_writable);

        tx.accounts.otherAccount.data[0] = 0xca;
        tx.accounts.otherSigner.data[1] = 0xfe;

        return tx.accounts.oneAccount.lamports;
    }
}