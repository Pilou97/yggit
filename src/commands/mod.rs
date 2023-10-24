pub mod push;
pub mod rebase;
pub mod show;

pub trait Execute {
    fn execute(&self) -> Result<(), ()>;
}
