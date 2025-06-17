#![cfg(feature = "integration")]

use std::fs::{self, create_dir_all, remove_dir_all};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use std::io::Read;

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

    fn new_piped(program: &str, args: &[&str]) -> Self {
        let command = Command::new(program)
            .args(args)
            .stderr(Stdio::piped())
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
    let filename = "send";
    let port = "6969";
    initialize(format!("{SERVER_DIR}/{filename}").as_str());

    let _server = CommandRunner::new("target/debug/tftpd", &["-p", port, "-d", SERVER_DIR]);
    thread::sleep(Duration::from_secs(1));
    let mut client = CommandRunner::new(
        "atftp",
        &[
            "-g",
            "-r",
            filename,
            "-l",
            format!("{CLIENT_DIR}/{filename}").as_str(),
            "127.0.0.1",
            port,
        ],
    );

    let status = client.wait();
    assert!(status.success());
}

#[test]
fn test_receive() {
    let filename = "receive";
    let port = "6970";
    initialize(format!("{CLIENT_DIR}/{filename}").as_str());

    let _server = CommandRunner::new("target/debug/tftpd", &["-p", port, "-d", SERVER_DIR]);
    thread::sleep(Duration::from_secs(1));
    let mut client = CommandRunner::new(
        "atftp",
        &[
            "-p",
            "-r",
            filename,
            "-l",
            format!("{CLIENT_DIR}/{filename}").as_str(),
            "127.0.0.1",
            port,
        ],
    );

    let status = client.wait();
    assert!(status.success());
}

#[test]
fn test_send_dir() {
    let filename = "send_dir";
    let port = "6971";
    initialize(format!("{SERVER_DIR}/{filename}").as_str());

    let _server = CommandRunner::new("target/debug/tftpd", &["-p", port, "-sd", SERVER_DIR]);
    thread::sleep(Duration::from_secs(1));
    let mut client = CommandRunner::new(
        "atftp",
        &[
            "-g",
            "-r",
            filename,
            "-l",
            format!("{CLIENT_DIR}/{filename}").as_str(),
            "127.0.0.1",
            port,
        ],
    );

    let status = client.wait();
    assert!(status.success());

    check_files(filename);
}

#[test]
fn test_receive_dir() {
    let filename = "receive_dir";
    let port = "6972";
    initialize(format!("{CLIENT_DIR}/{filename}").as_str());

    let _server = CommandRunner::new("target/debug/tftpd", &["-p", port, "-rd", SERVER_DIR]);
    thread::sleep(Duration::from_secs(1));
    let mut client = CommandRunner::new(
        "atftp",
        &[
            "-p",
            "-r",
            filename,
            "-l",
            format!("{CLIENT_DIR}/{filename}").as_str(),
            "127.0.0.1",
            port,
        ],
    );

    let status = client.wait();
    assert!(status.success());

    check_files(filename);
}

#[test]
fn test_send_ipv6() {
    let filename = "send_ipv6";
    let port = "6973";
    initialize(format!("{SERVER_DIR}/{filename}").as_str());

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
            filename,
            "-l",
            format!("{CLIENT_DIR}/{filename}").as_str(),
            "::1",
            port,
        ],
    );

    let status = client.wait();
    assert!(status.success());

    check_files(filename);
}

#[test]
fn test_receive_ipv6() {
    let filename = "receive_ipv6";
    let port = "6974";
    initialize(format!("{CLIENT_DIR}/{filename}").as_str());

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
            filename,
            "-l",
            format!("{CLIENT_DIR}/{filename}").as_str(),
            "::1",
            port,
        ],
    );

    let status = client.wait();
    assert!(status.success());

    check_files(filename);
}

#[test]
fn test_send_single_port_options() {
    let filename = "send_single_port_options";
    let port = "6975";
    initialize(format!("{SERVER_DIR}/{filename}").as_str());

    let _server = CommandRunner::new("target/debug/tftpd", &["-p", port, "-d", SERVER_DIR, "-s"]);
    thread::sleep(Duration::from_secs(1));
    let mut client = CommandRunner::new(
        "atftp",
        &[
            "-g",
            "-r",
            filename,
            "-l",
            format!("{CLIENT_DIR}/{filename}").as_str(),
            "--option",
            "windowsize 10",
            "127.0.0.1",
            port,
        ],
    );

    let status = client.wait();
    assert!(status.success());

    check_files(filename);
}

#[test]
fn test_client_send() {
    let filename = "client_send";
    let port = "6980";
    initialize(format!("{CLIENT_DIR}/{filename}").as_str());

    let _server = CommandRunner::new("target/debug/tftpd", &["-p", port, "-d", SERVER_DIR]);
    thread::sleep(Duration::from_secs(1));

    let mut client = CommandRunner::new(
        "target/debug/tftpc",
        &[
            format!("{CLIENT_DIR}/{filename}").as_str(),
            "-p",
            port,
            "-u",
        ],
    );

    let status = client.wait();
    assert!(status.success());

    check_files(filename);
}

#[test]
fn test_client_receive() {
    let filename = "client_receive";
    let port = "6981";
    initialize(format!("{SERVER_DIR}/{filename}").as_str());

    let _server = CommandRunner::new("target/debug/tftpd", &["-p", port, "-d", SERVER_DIR]);
    thread::sleep(Duration::from_secs(1));

    let mut client = CommandRunner::new(
        "target/debug/tftpc",
        &[filename, "-p", port, "-d", "-rd", CLIENT_DIR],
    );

    let status = client.wait();
    assert!(status.success());

    check_files(filename);
}

#[test]
fn test_client_receive_paths() {
    let filename = "client_receive_paths";
    let port = "6982";
    create_dir_all(format!("{SERVER_DIR}/subdir").as_str())
        .expect("error creating server directory");
    create_file(format!("{SERVER_DIR}/subdir/{filename}").as_str(), 10*1024*1024);

    let _server = CommandRunner::new("target/debug/tftpd", &["-p", port, "-d", SERVER_DIR]);
    thread::sleep(Duration::from_secs(1));

    let mut client = CommandRunner::new(
        "target/debug/tftpc",
        &[
            format!("subdir/{filename}").as_str(),
            "-p",
            port,
            "-d",
            "-rd",
            CLIENT_DIR,
        ],
    );

    let status = client.wait();
    assert!(status.success());

    let server_file = format!("{SERVER_DIR}/subdir/{filename}");
    let client_file = format!("{CLIENT_DIR}/{filename}");

    let server_content = fs::read(server_file).expect("error reading server file");
    let client_content = fs::read(client_file).expect("error reading client file");

    assert_eq!(server_content, client_content);
}

#[test]
fn test_client_receive_windows_paths() {
    let filename = "client_receive_windows_paths";
    let port = "6983";
    create_dir_all(format!("{SERVER_DIR}/windir").as_str())
        .expect("error creating server directory");
    create_file(format!("{SERVER_DIR}/windir/{filename}").as_str(), 10*1024*1024);

    let _server = CommandRunner::new("target/debug/tftpd", &["-p", port, "-d", SERVER_DIR]);
    thread::sleep(Duration::from_secs(1));

    let mut client = CommandRunner::new(
        "target/debug/tftpc",
        &[
            format!(r"windir\{filename}").as_str(),
            "-p",
            port,
            "-d",
            "-rd",
            CLIENT_DIR,
        ],
    );

    let status = client.wait();
    assert!(status.success());

    let server_file = format!("{SERVER_DIR}/windir/{filename}");
    let client_file = format!("{CLIENT_DIR}/{filename}");

    let server_content = fs::read(server_file).expect("error reading server file");
    let client_content = fs::read(client_file).expect("error reading client file");

    assert_eq!(server_content, client_content);
}

#[test]
fn test_send_curl() {
    let filename = "send_curl";
    let port = "6984";
    initialize(format!("{SERVER_DIR}/{filename}").as_str());

    // rollover=0, verbosity max, drop 1 data packet at half transfer
    let _server = CommandRunner::new("target/debug/tftpd", &["-p", port, "-d", SERVER_DIR, "-R", "0", "-v", "-v", "-D", "32768"]);
    thread::sleep(Duration::from_secs(1));
    let mut client = CommandRunner::new(
        "curl",
        &[
            "-O", //use remote filename locally
            "--output-dir", CLIENT_DIR,
            format!("tftp://127.0.0.1:{port}/{filename}").as_str(),
            "--tftp-blksize", "150", // blocksize to ensure 1 rollover
            "--connect-timeout", "3", // timeout = 1s
        ],
    );

    let status = client.wait();
    assert!(status.success());
}

#[test]
fn test_receive_curl() {
    let filename = "receive_curl";
    let port = "6985";
    initialize(format!("{CLIENT_DIR}/{filename}").as_str());

    // rollover=0, verbosity max, drop 1 data packet at half transfer
    let _server = CommandRunner::new("target/debug/tftpd", &["-p", port, "-d", SERVER_DIR, "-R", "0", "-v", "-v", "-D", "32768"]);
    thread::sleep(Duration::from_secs(1));
    let mut client = CommandRunner::new(
        "curl",
        &[
            "--upload-file",
            format!("{CLIENT_DIR}/{filename}").as_str(),
            format!("tftp://127.0.0.1:{port}/").as_str(),
            "--tftp-blksize", "150", // blocksize to ensure 1 rollover
            "--connect-timeout", "3", // timeout = 1s
        ],
    );

    let status = client.wait();
    assert!(status.success());
}

#[test]
fn test_rollover() {
    let filename = "rollover";
    let port = "6986";
    create_dir_all(format!("{SERVER_DIR}").as_str()).expect("error creating server directory");
    create_file(format!("{SERVER_DIR}/{filename}").as_str(), 65540);

    let _server = CommandRunner::new("target/debug/tftpd", &["-p", port, "-d", SERVER_DIR, "-R", "0", "-v", "-v", ]);
    thread::sleep(Duration::from_secs(1));

    let mut client = CommandRunner::new(
        "target/debug/tftpc",
        &[
            filename,
            "-p", port,
            "-d",
            "-rd", CLIENT_DIR,
            "-R", "0",
            "-b", "1", // speed up test and ensure rollover
            "-w", "32", // speed up test 
            "-v", "-v", 
        ],
    );

    let status = client.wait();
    assert!(status.success());

    let server_file = format!("{SERVER_DIR}/{filename}");
    let client_file = format!("{CLIENT_DIR}/{filename}");

    let server_content = fs::read(server_file).expect("error reading server file");
    let client_content = fs::read(client_file).expect("error reading client file");

    assert_eq!(server_content, client_content);
}

#[test]
fn test_rollover_fail() {
    let filename = "rollover_fail";
    let port = "6987";
    create_dir_all(format!("{SERVER_DIR}").as_str()).expect("error creating server directory");
    create_file(format!("{SERVER_DIR}/{filename}").as_str(), 65540);

    let _server = CommandRunner::new("target/debug/tftpd", &["-p", port, "-d", SERVER_DIR, "-R", "0", "-v", "-v", ]);
    thread::sleep(Duration::from_secs(1));

    let mut client = CommandRunner::new_piped(
        "target/debug/tftpc",
        &[
            filename,
            "-p", port,
            "-d",
            "-rd", CLIENT_DIR,
            "-R", "1", // different from server, must fail
            "-b", "1", // speed up test and ensure rollover
            "-w", "32", // speed up test
            "-v", "-v",
        ],
    );

    let mut stderr = client.process.stderr.take().unwrap();
    let status = client.wait();
    
    let mut buffer = String::new();
    let _ = stderr.read_to_string(&mut buffer);
    assert!(buffer.contains("Block counter rollover error"));
    
    assert!(!status.success());
}

// This test creates a file corruption by dropping "data 0" packet
// (the first one after a rollover) and ensuring transfer get terminated,
// then waits for the tsize check to detect the size mismatch
#[test]
fn test_tsize() {
    let filename = "tsize";
    let port = "6986";
    create_folders();
    create_file(format!("{SERVER_DIR}/{filename}").as_str(), 65540);

    let _server = CommandRunner::new("target/debug/tftpd", 
        &["-p", port, "-d", SERVER_DIR,
            "-R", "0", "-D", "0", // rollover 0 but drop data 0!
             "-v", "-v", 
        ]);
    thread::sleep(Duration::from_secs(1));

    let mut client = CommandRunner::new_piped(
        "target/debug/tftpc",
        &[
            filename,
            "-p", port,
            "-d",
            "-rd", CLIENT_DIR,
            "-R", "x",  // will not fail if data 0 is missing
            "-b", "1",  // speed up test and ensure rollover
            "-w", "30", // to ensure no ack on data 0
            "-v", "-v",
        ],
    );

    let mut stderr = client.process.stderr.take().unwrap();
    let status = client.wait();
    
    let mut buffer = String::new();
    let _ = stderr.read_to_string(&mut buffer);
    assert!(buffer.contains("Size mismatch"));
    assert!(!status.success());
}

#[test]
fn test_window() {
    let filename = "window";
    let port = "6988";
    initialize(format!("{SERVER_DIR}/{filename}").as_str());

    let _server = CommandRunner::new("target/debug/tftpd", &["-p", port, "-d", SERVER_DIR, "-v", "-v", "-D", "20458", ]);
    thread::sleep(Duration::from_secs(1));

    let now = Instant::now();
    let mut client = CommandRunner::new(
        "target/debug/tftpc",
        &[
            "-d", 
            filename, 
            "-p", port, 
            "-rd", CLIENT_DIR,
            "-w", "8",
            "-t", "10",
        ],
    );

    let status = client.wait();
    assert!(status.success());

    // Packet drop at window start should not trigger any timeout
    assert!(now.elapsed() < Duration::from_secs(10));
     
    check_files(filename);
}


#[test]
fn test_window_timeout() {
    let filename = "window_timeout";
    let port = "6989";
    initialize(format!("{SERVER_DIR}/{filename}").as_str());

    let _server = CommandRunner::new("target/debug/tftpd", &["-p", port, "-d", SERVER_DIR, "-v", "-v", "-D", "20464" ]);
    thread::sleep(Duration::from_secs(1));

    let now = Instant::now();
    let mut client = CommandRunner::new(
        "target/debug/tftpc",
        &[
            "-d", 
            filename, 
            "-p", port, 
            "-rd", CLIENT_DIR,
            "-w", "8",
            "-t", "6",
        ],
    );

    let status = client.wait();
    assert!(status.success());

    // Packet drop at window end should trigger one timeout
    assert!(now.elapsed() > Duration::from_secs(6));

    check_files(filename);
}


fn initialize(filename: &str) {
    create_folders();
    create_file(filename, 10*1024*1024);
}

fn create_folders() {
    let _ = remove_dir_all(SERVER_DIR);
    let _ = remove_dir_all(CLIENT_DIR);
    create_dir_all(SERVER_DIR).expect("error creating server directory");
    create_dir_all(CLIENT_DIR).expect("error creating client directory");
}

fn create_file(filename: &str, size: usize) {
    Command::new("dd")
        .args([
            "if=/dev/urandom",
            format!("of={filename}").as_str(),
            format!("bs={size}").as_str(),
            "count=1",
        ])
        .spawn()
        .expect("error creating test file")
        .wait()
        .expect("error waiting for test file creation");
}

fn check_files(filename: &str) {
    let server_file = format!("{SERVER_DIR}/{filename}");
    let client_file = format!("{CLIENT_DIR}/{filename}");

    let server_content = fs::read(server_file).expect("error reading server file");
    let client_content = fs::read(client_file).expect("error reading client file");

    assert_eq!(server_content, client_content);
}
