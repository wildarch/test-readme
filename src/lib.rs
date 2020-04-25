use std::borrow::Borrow;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};

use pulldown_cmark::{Event, Parser, Tag};
use thiserror::Error;

#[derive(Debug, Default, Clone)]
pub struct Options {
    extra_flags: HashMap<String, String>,
}

impl Options {
    pub fn flag(&mut self, tool: &str, flag: &str) -> &mut Self {
        let flag = " ".to_owned() + flag;
        if let Some(flags) = self.extra_flags.get_mut(tool) {
            flags.push_str(&flag);
        } else {
            // Always None
            let _ = self.extra_flags.insert(tool.to_owned(), flag.to_owned());
        }
        self
    }
}

pub fn build_markdown<P: AsRef<Path>, O: Borrow<Options>>(
    docker_base: &str,
    options: O,
    install_md: P,
) -> Result<(), Error> {
    let install_md_file = File::open(install_md).map_err(Error::InputIo)?;
    let mut commands = parse_commands(install_md_file)?;
    apply_extra_flags(&options.borrow().extra_flags, &mut commands);
    let dockerfile = Dockerfile {
        base: docker_base.into(),
        commands,
    };
    docker_build(&dockerfile)?;
    Ok(())
}

fn parse_commands(mut file: File) -> Result<Vec<String>, Error> {
    let mut text = String::new();
    file.read_to_string(&mut text).map_err(Error::InputIo)?;
    let parser = Parser::new(&text);

    let mut code = String::new();
    let mut inside_codeblock = false;
    for event in parser {
        match event {
            Event::Start(Tag::CodeBlock(_)) => {
                if inside_codeblock {
                    return Err(Error::ParseMd("Nested codeblock".into()));
                }
                inside_codeblock = true;
            }
            Event::End(Tag::CodeBlock(_)) => {
                inside_codeblock = false;
            }
            Event::Text(text) => {
                if inside_codeblock {
                    code.push_str(&text);
                }
            }
            _ => {}
        }
    }
    let commands = code.lines().map(String::from).collect();

    return Ok(commands);
}

fn apply_extra_flags(flags: &HashMap<String, String>, commands: &mut Vec<String>) {
    for command in commands.iter_mut() {
        for (prefix, flags) in flags.iter() {
            if command.starts_with(prefix) {
                command.insert_str(prefix.len(), &flags);
            }
        }
    }
}

#[derive(Debug, Clone)]
struct Dockerfile {
    base: String,
    commands: Vec<String>,
}

impl std::fmt::Display for Dockerfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "FROM {}", self.base)?;
        for command in &self.commands {
            writeln!(f, "RUN {}", command)?;
        }
        Ok(())
    }
}

fn docker_build(dockerfile: &Dockerfile) -> Result<(), Error> {
    let mut docker = Command::new("docker")
        .arg("build")
        // Read Dockerfile from stdin.
        .arg("-")
        .stdin(Stdio::piped())
        .spawn()
        .map_err(Error::DockerSpawn)?;
    let stdin = docker.stdin.as_mut().unwrap();
    writeln!(stdin, "{}", dockerfile).map_err(Error::DockerSpawn)?;

    let output = docker.wait_with_output().map_err(Error::DockerSpawn)?;
    if !output.status.success() {
        return Err(Error::DockerBuild(output.status));
    }

    Ok(())
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("I/O error reading input file: {0}")]
    InputIo(io::Error),
    #[error("Error parsing markdown: {0}")]
    ParseMd(String),
    #[error("Error spawning docker process: {0}")]
    DockerSpawn(io::Error),
    #[error("Docker failed with exit status: {0}")]
    DockerBuild(ExitStatus),
}
