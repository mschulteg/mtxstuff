use std::process::{self, ExitStatus};

#[derive(Debug)]
pub struct CommandOutput {
    pub status: ExitStatus,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug)]
pub struct Command {
    pub executable: String,
    pub arguments: Vec<String>,
    pub output: Option<CommandOutput>,
}

impl Command {
    pub fn new() -> Self {
        Command {
            executable: "mkvpropedit".into(),
            arguments: Vec::new(),
            output: None,
        }
    }

    pub fn to_cmd_string(&self) -> Option<String> {
        if self.arguments.len() == 0 {
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
        println!("Running command {:?}", command);
        let output = command.output()?;
        self.output = Some(CommandOutput {
            status: output.status,
            stdout: String::from_utf8(output.stdout).expect("commmand should return UTF8 data"),
            stderr: String::from_utf8(output.stderr).expect("commmand should return UTF8 data"),
        });
        Ok(())
    }
}
