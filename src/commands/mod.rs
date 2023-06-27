pub mod push;

pub trait Execute {
    fn execute(&self) -> Result<(), ()>;
}
