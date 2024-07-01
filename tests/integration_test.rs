#![cfg(feature = "integration")]

use std::fs::{create_dir_all, remove_dir_all};
use std::process::{Child, Command, ExitStatus};
use std::thread;
use std::time::Duration;

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
    let port = "6969";
    initialize(format!("{SERVER_DIR}/{file_name}").as_str());

    let _server = CommandRunner::new("target/debug/tftpd", &["-p", port, "-d", SERVER_DIR]);
    thread::sleep(Duration::from_secs(1));
    let mut client = CommandRunner::new(
        "atftp",
        &[
            "-g",
            "-r",
            file_name,
            "-l",
            format!("{CLIENT_DIR}/{file_name}").as_str(),
            "127.0.0.1",
            port,
        ],
    );

    let status = client.wait();
    assert!(status.success());
}

#[test]
fn test_receive() {
    let file_name = "receive";
    let port = "6970";
    initialize(format!("{CLIENT_DIR}/{file_name}").as_str());

    let _server = CommandRunner::new("target/debug/tftpd", &["-p", port, "-d", SERVER_DIR]);
    thread::sleep(Duration::from_secs(1));
    let mut client = CommandRunner::new(
        "atftp",
        &[
            "-p",
            "-r",
            file_name,
            "-l",
            format!("{CLIENT_DIR}/{file_name}").as_str(),
            "127.0.0.1",
            port,
        ],
    );

    let status = client.wait();
    assert!(status.success());
}

#[test]
fn test_send_dir() {
    let file_name = "send_dir";
    let port = "6971";
    initialize(format!("{SERVER_DIR}/{file_name}").as_str());

    let _server = CommandRunner::new("target/debug/tftpd", &["-p", port, "-sd", SERVER_DIR]);
    thread::sleep(Duration::from_secs(1));
    let mut client = CommandRunner::new(
        "atftp",
        &[
            "-g",
            "-r",
            file_name,
            "-l",
            format!("{CLIENT_DIR}/{file_name}").as_str(),
            "127.0.0.1",
            port,
        ],
    );

    let status = client.wait();
    assert!(status.success());

    check_files(file_name);
}

#[test]
fn test_receive_dir() {
    let file_name = "receive_dir";
    let port = "6972";
    initialize(format!("{CLIENT_DIR}/{file_name}").as_str());

    let _server = CommandRunner::new("target/debug/tftpd", &["-p", port, "-rd", SERVER_DIR]);
    thread::sleep(Duration::from_secs(1));
    let mut client = CommandRunner::new(
        "atftp",
        &[
            "-p",
            "-r",
            file_name,
            "-l",
            format!("{CLIENT_DIR}/{file_name}").as_str(),
            "127.0.0.1",
            port,
        ],
    );

    let status = client.wait();
    assert!(status.success());

    check_files(file_name);
}

#[test]
fn test_send_ipv6() {
    let file_name = "send_ipv6";
    let port = "6973";
    initialize(format!("{SERVER_DIR}/{file_name}").as_str());

    let _server = CommandRunner::new(
        "target/debug/tftpd",
        &["-i", "::1", "-p", port, "-d", SERVER_DIR],
    );
    thread::sleep(Duration::from_secs(1));
    let mut client = CommandRunner::new(
        "atftp",
        &[
            "-g",
            "-r",
            file_name,
            "-l",
            format!("{CLIENT_DIR}/{file_name}").as_str(),
            "::1",
            port,
        ],
    );

    let status = client.wait();
    assert!(status.success());

    check_files(file_name);
}

#[test]
fn test_receive_ipv6() {
    let file_name = "receive_ipv6";
    let port = "6974";
    initialize(format!("{CLIENT_DIR}/{file_name}").as_str());

    let _server = CommandRunner::new(
        "target/debug/tftpd",
        &["-i", "::1", "-p", port, "-d", SERVER_DIR],
    );
    thread::sleep(Duration::from_secs(1));
    let mut client = CommandRunner::new(
        "atftp",
        &[
            "-p",
            "-r",
            file_name,
            "-l",
            format!("{CLIENT_DIR}/{file_name}").as_str(),
            "::1",
            port,
        ],
    );

    let status = client.wait();
    assert!(status.success());

    check_files(file_name);
}

#[test]
fn test_send_single_port_options() {
    let file_name = "send_single_port_options";
    let port = "6975";
    initialize(format!("{SERVER_DIR}/{file_name}").as_str());

    let _server = CommandRunner::new("target/debug/tftpd", &["-p", port, "-d", SERVER_DIR, "-s"]);
    thread::sleep(Duration::from_secs(1));
    let mut client = CommandRunner::new(
        "atftp",
        &[
            "-g",
            "-r",
            file_name,
            "-l",
            format!("{CLIENT_DIR}/{file_name}").as_str(),
            "--option",
            "windowsize 10",
            "127.0.0.1",
            port,
        ],
    );

    let status = client.wait();
    assert!(status.success());

    check_files(file_name);
}

#[test]
fn test_client_send() {
    let file_name = "client_send";
    let port = "6980";
    initialize(format!("{CLIENT_DIR}/{file_name}").as_str());

    let _server = CommandRunner::new("target/debug/tftpd", &["-p", port, "-d", SERVER_DIR]);
    thread::sleep(Duration::from_secs(1));

    let mut client = CommandRunner::new(
        "target/debug/tftpc",
        &[
            format!("{CLIENT_DIR}/{file_name}").as_str(),
            "-p",
            port,
            "-u",
        ],
    );

    let status = client.wait();
    assert!(status.success());

    check_files(file_name);
}

#[test]
fn test_client_receive() {
    let file_name = "client_receive";
    let port = "6981";
    initialize(format!("{SERVER_DIR}/{file_name}").as_str());

    let _server = CommandRunner::new("target/debug/tftpd", &["-p", port, "-d", SERVER_DIR]);
    thread::sleep(Duration::from_secs(1));

    let mut client = CommandRunner::new(
        "target/debug/tftpc",
        &[
            file_name,
            "-p",
            port,
            "-d",
            "-rd",
            format!("{CLIENT_DIR}").as_str(),
        ],
    );

    let status = client.wait();
    assert!(status.success());

    check_files(file_name);
}

fn initialize(file_name: &str) {
    create_folders();
    create_file(file_name);
}

fn create_folders() {
    let _ = remove_dir_all(SERVER_DIR);
    let _ = remove_dir_all(CLIENT_DIR);
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

fn check_files(file_name: &str) {
    let server_file = format!("{SERVER_DIR}/{file_name}");
    let client_file = format!("{CLIENT_DIR}/{file_name}");

    let server_content = std::fs::read(server_file).expect("error reading server file");
    let client_content = std::fs::read(client_file).expect("error reading client file");

    assert_eq!(server_content, client_content);
}
