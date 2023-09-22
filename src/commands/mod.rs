pub mod push;
pub mod show;
pub mod test;

pub trait Execute {
    fn execute(&self) -> Result<(), ()>;
}
