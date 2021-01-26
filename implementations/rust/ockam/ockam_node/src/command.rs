#[derive(Clone, Debug)]
pub struct StopCommand;

impl StopCommand {
    pub fn run(&self) {
        println!("stopping");
    }
}

#[derive(Clone, Debug)]
pub enum Command {
    Stop(StopCommand),
}

impl Command {
    pub fn stop() -> Command {
        Command::Stop(StopCommand {})
    }
}
