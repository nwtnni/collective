pub mod ifstat;
pub mod osu;

pub use ifstat::Ifstat;
pub use osu::Osu;

pub trait Plot {
    fn plot(self) -> anyhow::Result<()>;
}
