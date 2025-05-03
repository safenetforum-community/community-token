use autonomi::PublicKey;


//pub fn 


#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn pk_cannot_be_arbitrary_32bytes() -> Result<(), String> {
		// EVM txid
		let pubkey = PublicKey::from_hex("91c680f29bb12c72093642aa6750332e140753bd112097e021428d86b12ee479");
		assert!(pubkey.is_err());

		// EVM address, 20 bytes
		let pubkey = PublicKey::from_hex("a78d8321b20c4ef90ecd72f2588aa985a4bdb684");
		assert!(pubkey.is_err());

		// 32 bytes
		let pubkey = PublicKey::from_hex("a78d8321b20c4ef90ecd72f2588aa985a4bdb684000000000000000000000000");
		assert!(pubkey.is_err());

		Ok(())
	}

	#[test]
	fn can_reference_erc20() {
		// TODO
		assert_eq!(0, 0);
	}
}
