//! SPDX-License-Identifier: (Apache-2.0 OR MIT)

#[derive(Debug, Copy, Clone, Default)]
pub struct FpState {}

impl FpState {
    pub fn new() -> Self {
        Self { ..Self::default() }
    }

    pub fn save(&mut self) {}

    pub fn restore(&self) {}
}
