#![cfg_attr(not(feature = "std"), no_std)]
extern crate runtime_primitives;
extern crate client;

use client::decl_runtime_apis;
use runtime_primitives::traits::NumberFor;

decl_runtime_apis! {
	pub trait LnApi {
		fn link_bridge(number: NumberFor<Block>);
	}
}
