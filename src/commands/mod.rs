pub mod push;
pub mod show;

pub trait Execute {
    fn execute(&self) -> Result<(), ()>;
}
