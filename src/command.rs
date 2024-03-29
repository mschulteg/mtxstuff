use std::process::{self, ExitStatus};
use std::sync::mpsc;
use std::thread;
use std::thread::JoinHandle;

#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub status: ExitStatus,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone)]
pub struct Command {
    pub executable: String,
    pub arguments: Vec<String>,
    pub output: Option<CommandOutput>,
}

impl Command {
    pub fn new(executable: impl AsRef<str>) -> Self {
        Command {
            executable: executable.as_ref().into(),
            arguments: Vec::new(),
            output: None,
        }
    }

    // pub fn to_log_string(&self) -> String {
    //     let mut message = format!("Command: {:}\n", self.to_cmd_string().unwrap());
    //     if let Some(ref output) = self.output {
    //         if output.status.success() {
    //             message.push_str("Success");
    //         } else {
    //             message.push_str(&format!("Error: {:}", output.status));
    //         }
    //     } else {
    //         message.push_str("Has not run.");
    //     }
    //     message
    // }

    pub fn success_string(&self) -> String {
        let mut message = String::new();
        if let Some(ref output) = self.output {
            if output.status.success() {
                message.push_str("Success");
            } else {
                message.push_str(&format!("Error: {:}", output.status));
            }
        } else {
            message.push_str("Has not run.");
        }
        message
    }

    pub fn to_cmd_string(&self) -> Option<String> {
        // Do not check this here because command is supposed to be universal
        if self.arguments.is_empty() {
            return None;
        }
        let mut string = self.executable.clone();
        //string.push_str(&self.arguments.join(" "));
        for argument in &self.arguments {
            string.push(' ');
            if argument.contains(char::is_whitespace) {
                string.push('"');
                string.push_str(argument);
                string.push('"');
            } else {
                string.push_str(argument);
            }
        }
        Some(string)
    }

    pub fn run(&mut self) -> std::io::Result<()> {
        let mut command = process::Command::new(&self.executable);
        let command = command.args(&self.arguments);
        //println!("Running command {:?}", command);
        let output = command.output()?;
        self.output = Some(CommandOutput {
            status: output.status,
            stdout: String::from_utf8(output.stdout).expect("commmand should return UTF8 data"),
            stderr: String::from_utf8(output.stderr).expect("commmand should return UTF8 data"),
        });
        Ok(())
    }
}

pub(crate) struct CommandHandler {
    command_thread: JoinHandle<()>,
    result_receiver: mpsc::Receiver<std::io::Result<Command>>,
    done_commands: Vec<std::io::Result<Command>>,
    num_commands: usize,
}

#[derive(Clone, Copy)]
pub(crate) enum CommandHandlerStatus {
    Percent(u16),
    Done,
}

impl CommandHandler {
    pub(crate) fn new(commands: Vec<Command>) -> Self {
        let (tx_cmd, rx_cmd): (mpsc::Sender<Command>, mpsc::Receiver<Command>) = mpsc::channel();
        let (tx_res, rx_res): (
            mpsc::Sender<std::io::Result<Command>>,
            mpsc::Receiver<std::io::Result<Command>>,
        ) = mpsc::channel();
        let command_thread = thread::spawn(move || loop {
            let task = rx_cmd.recv();
            match task {
                Ok(mut command) => {
                    match command.run() {
                        Ok(_) => tx_res.send(Ok(command)).unwrap(),
                        Err(err) => tx_res.send(Err(err)).unwrap(),
                    };
                }
                Err(_) => break, // producer is gone, we are done,
            }
        });
        let num_commands = commands.len();
        for command in commands {
            tx_cmd.send(command).unwrap();
        }

        Self {
            command_thread,
            result_receiver: rx_res,
            done_commands: Default::default(),
            num_commands,
        }
    }

    pub(crate) fn check(&mut self) -> CommandHandlerStatus {
        while let Ok(received) = self.result_receiver.try_recv() {
            self.done_commands.push(received)
        }
        if self.num_commands == self.done_commands.len() {
            CommandHandlerStatus::Done
        } else {
            let ratio = self.done_commands.len() as f64 / self.num_commands as f64;
            CommandHandlerStatus::Percent((ratio * 100f64).round() as u16)
        }
    }

    pub(crate) fn into_results(self) -> Vec<std::io::Result<Command>> {
        self.command_thread.join().unwrap();
        self.done_commands
    }
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_command_handler() {
        let mut commands = Vec::<Command>::new();
        for i in 0..5 {
            let mut command = Command::new("sleep");
            command.arguments.push((i as f64 / 10.0).to_string());
            commands.push(command);
        }

        let mut command_handler = CommandHandler::new(commands);
        loop {
            match command_handler.check() {
                CommandHandlerStatus::Percent(percent) => {
                    println!("{:}%", percent)
                }
                CommandHandlerStatus::Done => {
                    println!("done");
                    break;
                }
            }
            thread::sleep(std::time::Duration::from_millis(100));
        }
        for command in command_handler.into_results() {
            let command = command.expect("done without error");
            let output = command.output.expect("command has run so there is output");
            assert!(output.status.success());
        }
    }
}
