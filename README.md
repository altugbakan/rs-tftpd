# TFTP Server Daemon

Pure [Rust](https://www.rust-lang.org/) implementation of a Trivial File Transfer Protocol server daemon.

This server implements [RFC 1350](https://www.rfc-editor.org/rfc/rfc1350), The TFTP Protocol (Revision 2). It also supports the following [RFC 2347](https://www.rfc-editor.org/rfc/rfc2347) TFTP Option Extensions:

- [RFC 2348](https://www.rfc-editor.org/rfc/rfc2348) Blocksize Option
- [RFC 2349](https://www.rfc-editor.org/rfc/rfc2349) Timeout Interval Option
- [RFC 2349](https://www.rfc-editor.org/rfc/rfc2349) Transfer Size Option
- [RFC 7440](https://www.rfc-editor.org/rfc/rfc7440) Windowsize Option

# Security

Since TFTP servers do not offer any type of login or access control mechanisms, this server only allows transfer and receiving inside a chosen folder, and disallows external file access.

# Documentation

Documentation for the project can be found in [docs.rs](https://docs.rs/tftpd/latest/tftpd/).

# Usage

To install the server using Cargo:

```bash
cargo install tftpd
tftpd --help
```

To run the server on the IP address `0.0.0.0`, port `1234` in the `/home/user/tftp` directory:

```bash
tftpd -i 0.0.0.0 -p 1234 -d "/home/user/tftp"
```

# License

This project is licensed under the [MIT License](https://opensource.org/license/mit/).
