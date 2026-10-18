// SPDX-License-Identifier: Apache-2.0
#![doc = include_str!("../README.md")]

mod calendar;

pub use calendar::{Almanac, Calendar, CalendarError, CalendarEvent, Season, SeasonId};
