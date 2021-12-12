use suborbital::runnable::*;
use suborbital::util;

struct HelloworldRs{}

impl Runnable for HelloworldRs {
    fn run(&self, input: Vec<u8>) -> Result<Vec<u8>, RunErr> {
        let msg = format!("hello {}", util::to_string(input));

        Ok(util::to_vec(String::from(msg)))
    }
}


// initialize the runner, do not edit below //
static RUNNABLE: &HelloworldRs = &HelloworldRs{};

#[no_mangle]
pub extern fn _start() {
    use_runnable(RUNNABLE);
}
