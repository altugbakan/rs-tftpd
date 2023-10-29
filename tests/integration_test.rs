// #![cfg(feature = "integration")]

use std::fs::create_dir_all;
use std::process::{Child, Command, ExitStatus};

const SERVER_DIR: &str = "target/integration/server";
const CLIENT_DIR: &str = "target/integration/client";

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
    let file_name = "send";
    initialize(format!("{SERVER_DIR}/{file_name}").as_str());

    let _server = CommandRunner::new("target/debug/tftpd", &["-p", "6969", "-d", SERVER_DIR]);
    let mut client = CommandRunner::new(
        "time",
        &["atftp", "-g", "-r", file_name, "127.0.0.1", "6969"],
    );

    let status = client.wait();
    assert!(status.success());
}

#[test]
fn test_receive() {
    let file_name = "receive";
    initialize(format!("{CLIENT_DIR}/{file_name}").as_str());

    let _server = CommandRunner::new("target/debug/tftpd", &["-p", "6970", "-d", SERVER_DIR]);
    let mut client = CommandRunner::new(
        "time",
        &[
            "atftp",
            "-p",
            "-l",
            format!("{CLIENT_DIR}/{file_name}").as_str(),
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
    create_dir_all(SERVER_DIR).expect("error creating server directory");
    create_dir_all(CLIENT_DIR).expect("error creating client directory");
}

fn create_file(file_name: &str) {
    Command::new("dd")
        .args([
            "if=/dev/urandom",
            format!("of={file_name}").as_str(),
            "bs=1M",
            "count=10",
        ])
        .spawn()
        .expect("error creating test file")
        .wait()
        .expect("error waiting for test file creation");
}
