pub trait Bot {
    unsafe fn plugin(&mut self, filename: &str) -> Result<(), PluginError>;
    fn start(&mut self);
}

#[derive(Debug)]
pub enum PluginError {
    InvalidLibrary,
    InvalidPluginConstructor
}