pub mod ifstat;

pub use ifstat::Ifstat;

pub trait Plot {
    fn plot(self) -> anyhow::Result<()>;
}
