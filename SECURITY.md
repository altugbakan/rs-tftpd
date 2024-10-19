# Security Policy
The TFTP Server Daemon project takes security bugs in this repository seriously. Your efforts to responsibly disclose your findings is appreciated, and we will make every effort to acknowledge your contributions.

## Reporting a Vulnerability
To report a security issue, please use the GitHub Security Advisory ["Report a Vulnerability"](https://github.com/altugbakan/rs-tftpd/security/advisories/new) tab on our GitHub repository.

Please **DO NOT** use public channels (e.g., GitHub issues) for initial reporting of bona fide security vulnerabilities.
Once you report a security issue, a reviewer will respond with the next steps. After the initial reply, you will be kept informed of the progress towards a fix and any forthcoming announcements. The reviewer may ask for additional information or guidance during this process. If we determine that your report does not constitute a genuine security vulnerability, you will be informed and the report will be closed. Your report may be turned into an issue for further tracking.

## Security Recommendations
Since TFTP lacks login or access control mechanisms, the server limits file transfers to a designated folder. It is highly recommended to run the server in a secure and isolated environment to prevent unauthorized file access and to only enable read-only mode if file uploads are not required.

Thank you for helping us keep the TFTP Server Daemon project secure!
