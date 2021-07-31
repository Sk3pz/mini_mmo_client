use hashbrown::HashMap;
use std::env::args;

pub enum CommandFailReason {
    InvalidCommand,
    NoCommandGiven,
}

pub struct CommandMuncher<T> {
    commands: HashMap<String, Box<dyn Fn(Vec<String>) -> T>>,
}

impl<T> CommandMuncher<T> {
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
        }
    }

    pub fn register<F: 'static + Fn(Vec<String>) -> T, S: Into<String>>(&mut self, command: S, handler: F) {
        self.commands.insert(command.into(), Box::new(handler));
    }

    pub fn unregister<S: Into<String>>(&mut self, command: S) {
        self.commands.remove(&(command.into()));
    }

    pub fn munch<S: Into<String>>(&mut self, input: S) -> Result<T, String> {
        let full = input.into(); // convert the input to a string
        if full.is_empty() { // ensure there is an actual command to nom
            return Err(format!("Can not process an empty command!"));
        }
        let split = full.split(" "); // split the command by whitespace
        let args_strs = split.collect::<Vec<&str>>(); // convert to a vector
        let mut args = args_strs.iter().map(|s| s.to_string()).collect::<Vec<String>>();
        let command = args.remove(0); // get the command and remove it from args

        let closure = self.commands.get(&command); // attempt to get the closure
        if closure.is_none() { // if there was no closure found, fail
            return Err(format!("The command '{}' does not exist!", command));
        }

        let cl = closure.unwrap(); // get the closure to run
        Ok(cl(args)) // return the result of the closure
    }
}