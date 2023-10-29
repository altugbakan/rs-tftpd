#![cfg(feature = "integration")]

use std::process::{Child, Command, ExitStatus};

const SOURCE_DIR: &str = "target/integration/source";
const DESTINATION_DIR: &str = "target/integration/destination";
const FILE_PREFIX: &str = "10M_FILE";

struct CommandRunner {
    process: Child,
}

impl CommandRunner {
    fn new(program: &str, args: &[&str]) -> Self {
        let command = Command::new(program)
            .args(args)
            .spawn()
            .expect("error starting process");
        Self { process: command }
    }

    fn wait(&mut self) -> ExitStatus {
        self.process.wait().expect("error waiting for process")
    }

    fn kill(&mut self) {
        self.process.kill().expect("error killing process");
    }
}

impl Drop for CommandRunner {
    fn drop(&mut self) {
        self.kill()
    }
}

#[test]
fn test_send() {
    let file_name = format!("{FILE_PREFIX}_send");
    initialize(file_name.as_str());

    let _server = CommandRunner::new("target/debug/tftpd", &["-p", "6969", "-d", SOURCE_DIR]);
    let mut client = CommandRunner::new(
        "time",
        &[
            "atftp",
            "-g",
            "-r",
            format!("{SOURCE_DIR}/{file_name}").as_str(),
            "-l",
            format!("{DESTINATION_DIR}/{file_name}").as_str(),
            "127.0.0.1",
            "6969",
        ],
    );

    let status = client.wait();
    assert!(status.success());
}

#[test]
fn test_receive() {
    let file_name = format!("{FILE_PREFIX}_receive");
    initialize(file_name.as_str());

    let _server = CommandRunner::new("target/debug/tftpd", &["-p", "6970", "-d", SOURCE_DIR]);
    let mut client = CommandRunner::new(
        "time",
        &[
            "atftp",
            "-p",
            "-r",
            format!("{DESTINATION_DIR}/{file_name}").as_str(),
            "-l",
            format!("{SOURCE_DIR}/{file_name}").as_str(),
            "127.0.0.1",
            "6969",
        ],
    );

    let status = client.wait();
    assert!(status.success());
}

fn initialize(file_name: &str) {
    create_folders();
    create_file(file_name);
}

fn create_folders() {
    Command::new("mkdir")
        .args(["-p", SOURCE_DIR])
        .spawn()
        .expect("error creating source directory");
    Command::new("mkdir")
        .args(["-p", DESTINATION_DIR])
        .spawn()
        .expect("error creating destionation directory");
}

fn create_file(file_name: &str) {
    Command::new("dd")
        .args([
            "if=/dev/urandom",
            format!("of={SOURCE_DIR}/{file_name}").as_str(),
            "bs=1M",
            "count=10",
        ])
        .spawn()
        .expect("error creating test file");
}
