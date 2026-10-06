// SPDX-License-Identifier: Apache-2.0
//! Spikes: small experiments that ask whether a design holds before anything builds on it.

// Native only: the spike takes seconds optimised and a minute without.
#[cfg(not(target_arch = "wasm32"))]
mod two_hundred_partners;
