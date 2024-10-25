#[derive(Debug, defmt::Format)]
pub struct DeviceId(&'static [u8; 12]);

impl riot_rs_embassy_common::identity::DeviceId for DeviceId {
    type Bytes = &'static [u8; 12];

    #[allow(
        refining_impl_trait_reachable,
        reason = "Making this fallible would be a breaking API change for RIOT-rs."
    )]
    fn get() -> Result<Self, core::convert::Infallible> {
        Ok(Self(embassy_stm32::uid::uid()))
    }

    fn bytes(&self) -> Self::Bytes {
        self.0
    }
}
