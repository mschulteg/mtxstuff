use std::process;

use super::file::File;

#[derive(Debug)]
pub struct Command<'a> {
    pub executable: String,
    pub file: &'a File,
    pub arguments: Vec<String>,
}

impl<'a> Command<'a> {
    pub fn new(file: &'a File) -> Self {
        Command {
            executable: "mkvpropedit".into(),
            file,
            arguments: Vec::new(),
        }
    }

    pub fn to_cmd_string(&self) -> Option<String> {
        if self.arguments.len() == 0 {
            return None;
        }
        let mut string = self.executable.clone();
        string.push(' ');
        string.push_str(&self.arguments.join(" "));
        string.push(' ');
        string.push('"');
        string.push_str(&self.file.file_name);
        string.push('"');
        Some(string)
    }

    pub fn run(&self) {
        let mut string = self.executable.clone();
        for arg in self.arguments.iter() {
            string.push(' ');
            string.push_str(arg);
        }
        //println!("Running command {:?}", string);
        let mut command = process::Command::new(&self.executable);
        let command = command
                .args(&self.arguments)
                .arg(&self.file.file_name);
        println!("Running command {:?}", command);
        let output = command
                .output()
                .unwrap();
        let stdout = String::from_utf8(output.stdout).unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();
        println!("stdout was:{}", &stdout);
        println!("stderr was:{}", &stderr);
        println!("status: {}", output.status);
    }
}
