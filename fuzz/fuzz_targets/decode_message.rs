// SPDX-License-Identifier: Apache-2.0
#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| lockstep_fuzz_guided::run("decode_message", data));
