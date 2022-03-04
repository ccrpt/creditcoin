use crate::{
	pallet::*,
	types::{Address, AddressId},
	DealOrderId, Error, Guid, Id, TransferId,
};
use frame_support::ensure;
use frame_system::pallet_prelude::*;
use sp_io::hashing::sha2_256;
use sp_runtime::{traits::UniqueSaturatedInto, RuntimeAppPublic};
use sp_std::prelude::*;

#[allow(unused_macros)]
macro_rules! try_get {
	($storage: ident <$t: ident>, $key: expr, $err: ident) => {
		crate::pallet::$storage::<$t>::try_get($key).map_err(|()| crate::pallet::Error::<$t>::$err)
	};
}

macro_rules! try_get_id {
	($storage: ident <$t: ident>, $key: expr, $err: ident) => {
		<crate::pallet::$storage<$t> as DoubleMapExt<_, _, _, _, _, _, _, _, _, _>>::try_get_id(
			$key,
		)
		.map_err(|()| crate::pallet::Error::<$t>::$err)
	};
}

type DealOrderFor<T> = crate::DealOrder<
	<T as frame_system::Config>::AccountId,
	<T as frame_system::Config>::BlockNumber,
	<T as frame_system::Config>::Hash,
	<T as pallet_timestamp::Config>::Moment,
>;
type TransferFor<T> = crate::Transfer<
	<T as frame_system::Config>::AccountId,
	<T as frame_system::Config>::BlockNumber,
	<T as frame_system::Config>::Hash,
>;

impl<T: Config> Pallet<T> {
	pub fn block_number() -> BlockNumberFor<T> {
		<frame_system::Pallet<T>>::block_number()
	}
	pub fn timestamp() -> T::Moment {
		<pallet_timestamp::Pallet<T>>::get()
	}
	pub fn get_address(address_id: &AddressId<T::Hash>) -> Result<Address<T::AccountId>, Error<T>> {
		Self::addresses(&address_id).ok_or(Error::<T>::NonExistentAddress)
	}

	pub fn authority_id() -> Option<T::AccountId> {
		let local_keys = crate::crypto::Public::all()
			.into_iter()
			.map(|p| sp_core::sr25519::Public::from(p).into())
			.collect::<Vec<T::FromAccountId>>();

		log::trace!("{:?}", local_keys);

		Authorities::<T>::iter_keys().find_map(|auth| {
			let acct = auth.clone().into();
			local_keys.contains(&acct).then(|| auth)
		})
	}

	pub fn register_deal_order_message(
		expiration_block: T::BlockNumber,
		ask_guid: &Guid,
		bid_guid: &Guid,
	) -> [u8; 32] {
		let expiration_block_u64: u64 = expiration_block.unique_saturated_into();
		let mut buf = lexical::to_string(expiration_block_u64).into_bytes();
		let block_end_idx = buf.len();
		buf.reserve(2 * (ask_guid.len() + bid_guid.len()));
		buf.extend(core::iter::repeat(0u8).take(2 * ask_guid.len()));
		hex::encode_to_slice(&*ask_guid, &mut buf[block_end_idx..])
			.expect("we allocated 2 * (length of guid) bytes, it must be enough capacity; qed");
		buf.extend(core::iter::repeat(0u8).take(2 * bid_guid.len()));
		hex::encode_to_slice(&*bid_guid, &mut buf[(block_end_idx + 2 * ask_guid.len())..])
			.expect("we just allocated 2 * (length of guid) bytes; qed");
		sha2_256(&buf)
	}

	pub fn try_mutate_deal_order_and_transfer(
		deal_order_id: &DealOrderId<T::BlockNumber, T::Hash>,
		transfer_id: &TransferId<T::Hash>,
		mutate_deal: impl FnOnce(
			&mut DealOrderFor<T>,
		) -> Result<Option<crate::Event<T>>, crate::Error<T>>,
		mutate_transfer: impl FnOnce(
			&mut TransferFor<T>,
			&DealOrderFor<T>,
		) -> Result<Option<crate::Event<T>>, crate::Error<T>>,
	) -> Result<(), crate::Error<T>> {
		let result = DealOrders::<T>::try_mutate(
			deal_order_id.expiration(),
			deal_order_id.hash(),
			|value| {
				let deal_order = value.as_mut().ok_or(crate::Error::<T>::NonExistentDealOrder)?;
				let deal_event = mutate_deal(deal_order)?;

				let transfer_event = Transfers::<T>::try_mutate(transfer_id, |value| {
					let transfer = value.as_mut().ok_or(crate::Error::<T>::NonExistentTransfer)?;
					mutate_transfer(transfer, deal_order)
				})?;

				Ok((deal_event, transfer_event))
			},
		);

		match result {
			Ok((deal_event, transfer_event)) => {
				if let Some(event) = deal_event {
					Self::deposit_event(event);
				}
				if let Some(event) = transfer_event {
					Self::deposit_event(event)
				}

				Ok(())
			},
			Err(e) => Err(e),
		}
	}

	pub fn use_guid(guid: &Guid) -> Result<(), Error<T>> {
		ensure!(!<UsedGuids<T>>::contains_key(guid.clone()), Error::<T>::GuidAlreadyUsed);
		UsedGuids::<T>::insert(guid, ());
		Ok(())
	}
}

mod tests {
	#[test]
	fn register_deal_order_message_works() {
		use core::convert::TryFrom;
		use frame_support::BoundedVec;
		let expiration_block = 5;
		let ask_guid = BoundedVec::try_from(b"asdfasdfasdfasdf".to_vec()).unwrap();
		let bid_guid = BoundedVec::try_from(b"qwerqwerqwerqwer".to_vec()).unwrap();
		let expected = b"\xf0hQ\xa9<yX\r\xcf\xfb\xf5\xf8\xe3\x94/\xd5\xff!\x86O+\xc0\x8e\xff\x9a\x9eH\xdf\xc3\x1f\x15\xc8";
		assert_eq!(
			&crate::Pallet::<crate::mock::Test>::register_deal_order_message(
				expiration_block,
				&ask_guid,
				&bid_guid,
			),
			expected
		);
	}

	#[test]
	fn register_deal_order_message_empty_guids() {
		use core::convert::TryFrom;
		use frame_support::BoundedVec;
		let expiration_block = 5;
		let ask_guid = BoundedVec::try_from(vec![]).unwrap();
		let bid_guid = BoundedVec::try_from(vec![]).unwrap();
		let expected = b"\xef-\x12}\xe3{\x94+\xaa\xd0aE\xe5K\x0ca\x9a\x1f\"2{.\xbb\xcf\xbe\xc7\x8fUd\xaf\xe3\x9d";
		assert_eq!(
			&crate::Pallet::<crate::mock::Test>::register_deal_order_message(
				expiration_block,
				&ask_guid,
				&bid_guid,
			),
			expected
		);
	}
}
