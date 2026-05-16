use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoEnumIterator};

#[derive(Debug, Serialize, Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct Cook {
	pub name: String,
	#[serde(default)]
	pub skills: HashMap<String, SkillLevel>,
}

#[derive(
	Debug,
	Clone,
	Copy,
	PartialEq,
	Eq,
	Hash,
	PartialOrd,
	Ord,
	EnumIter,
	Serialize,
	Deserialize,
	ts_rs::TS,
)]
#[repr(u8)]
#[ts(export)]
pub enum SkillLevel {
	Unskilled = 0,
	Novice,
	Intermediate,
	Advanced,
	Expert,
}

pub(crate) fn duration_for_skill(map: &HashMap<SkillLevel, u32>, level: SkillLevel) -> Option<u32> {
	SkillLevel::iter()
		.rev()
		.filter(|l| *l <= level)
		.find_map(|l| map.get(&l).copied())
}

#[cfg(test)]
mod tests {
	use googletest::prelude::*;

	use super::*;

	#[test]
	fn exact_match_returns_that_value() {
		let map = HashMap::from([(SkillLevel::Novice, 10), (SkillLevel::Advanced, 7)]);
		assert_that!(duration_for_skill(&map, SkillLevel::Advanced), eq(Some(7)));
	}

	#[test]
	fn falls_back_to_lower_level_when_exact_not_found() {
		let map = HashMap::from([(SkillLevel::Novice, 10)]);
		assert_that!(duration_for_skill(&map, SkillLevel::Advanced), eq(Some(10)));
	}

	#[test]
	fn returns_none_when_no_level_in_range() {
		let map = HashMap::from([(SkillLevel::Advanced, 7)]);
		assert_that!(duration_for_skill(&map, SkillLevel::Novice), eq(None));
	}

	#[test]
	fn prefers_closest_lower_level() {
		let map = HashMap::from([
			(SkillLevel::Unskilled, 20),
			(SkillLevel::Intermediate, 10),
			(SkillLevel::Expert, 5),
		]);
		assert_that!(duration_for_skill(&map, SkillLevel::Advanced), eq(Some(10)));
	}

	#[test]
	fn empty_map_returns_none() {
		assert_that!(
			duration_for_skill(&HashMap::new(), SkillLevel::Intermediate),
			eq(None)
		);
	}
}
