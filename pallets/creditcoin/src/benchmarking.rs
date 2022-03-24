#![cfg(feature = "runtime-benchmarks")]
//! Benchmarking setup for pallet-template

use super::*;

#[allow(unused)]
use crate::Pallet as Creditcoin;
use pallet_balances::Pallet as Balances;
use frame_benchmarking::{benchmarks, impl_benchmark_test_suite, whitelisted_caller,account,whitelist_account,Zero};
use frame_support::{pallet_prelude::*,traits::Currency};
use frame_system::RawOrigin;
use sp_core::ecdsa;
use sp_runtime::traits::IdentifyAccount;

#[extend::ext]
impl<'a, S> &'a [u8]
where
	S: Get<u32>,
{
	fn try_into_bounded(self) -> Result<BoundedVec<u8, S>, ()> {
		core::convert::TryFrom::try_from(self.to_vec())
	}
	fn into_bounded(self) -> BoundedVec<u8, S> {
		core::convert::TryFrom::try_from(self.to_vec()).unwrap()
	}
}

benchmarks! {
	register_address {
		let caller: T::AccountId = whitelisted_caller();
		let b in 0..256;
		let e in 0..256;
		let blockchain = vec![b'b'; b as usize];
		let external_address = vec![b'a'; e as usize];
	}: _(RawOrigin::Signed(caller), Blockchain::Other(blockchain.into_bounded()), external_address.into_bounded())

		//on_initialize{}:{}
		//a,b,c,d
		//on_finalize{}:{}
	claim_legacy_wallet {
		let pubkey = {
			let raw_key:[u8;33]= hex::decode("0399d6e7c784494fd7edc26fc9ca460a68c97cc64c49c85dfbb68148f0607893bf").unwrap().try_into().unwrap();
			ecdsa::Public::from_raw(raw_key)
		};

		let claimer = T::Signer::from(pubkey.clone()).into_account();
		whitelist_account!(claimer);

		let sighash = LegacySighash::from(&pubkey);
		let cash = <Balances<T> as Currency<T::AccountId>>::minimum_balance();
		LegacyWallets::<T>::insert(sighash, cash.clone());

		let keeper: T::AccountId = account("keeper", 1, 1);
		<Balances<T> as Currency<T::AccountId>>::make_free_balance_be(&keeper,cash);

		LegacyBalanceKeeper::<T>::put(keeper.clone());

	}: _(RawOrigin::Signed(claimer.clone()), pubkey)
	verify {
		assert!(Balances::<T>::free_balance(&keeper.clone()).is_zero());
		assert_eq!(Balances::<T>::free_balance(&claimer),cash);

	}
		//add_ask_order{}:{}
		//add_bid_order{}:{}
		//add_offer{}:{}
		//add_deal_order{}:{}
		//lock_deal_order{}:{}
		//fund_deal_order{}:{}
		//register_deal_order{}:{}
		//close_deal_order{}:{}
		//register_transfer{}:{}
		//excempt{}:{}
		//verify_transfer{}:{}
		//add_authority{}:{}
}

impl_benchmark_test_suite!(Creditcoin, crate::mock::new_test_ext(), crate::mock::Test);
