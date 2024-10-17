# Security Policy
The TFTP Server Daemon project team and community take security bugs in this repository seriously. We appreciate your efforts to responsibly disclose your findings, and will make every effort to acknowledge your contributions.

## Reporting a Vulnerability
To report a security issue, please use one of the following methods:
- **GitHub Security Advisory:** Use the GitHub Security Advisory ["Report a Vulnerability"](https://github.com/altugbakan/rs-tftpd/security/advisories/new) tab on our GitHub repository.
- **Send an Email:** Send your report to mail@alt.ug. We recommend encrypting the email if possible.

Please **DO NOT** use public channels (e.g., GitHub issues) for initial reporting of bona fide security vulnerabilities.
Once you report a security issue, our team will respond with the next steps. After our initial reply, we will keep you informed of the progress towards a fix and full announcement. We may ask for additional information or guidance during this process. If we disagree that your report constitutes a genuine security vulnerability, we will inform you and close the report. Your report may be turned into an issue for further tracking.

## Security Issues in Third-Party Dependencies
If you discover vulnerabilities in third-party modules used by this project, please report them to the maintainers of the respective modules. If the vulnerability impacts TFTP Server Daemon project directly, we encourage you to notify us using the above methods. We will validate if the vulnerability is exploitable from repository code; please note that not all vulnerabilities are actually exploitable and do not constitute an immediate concern for the project.

## Security Recommendations
Since TFTP lacks login or access control mechanisms, the server limits file transfers to a designated folder. It is highly recommended to run the server in a secure and isolated environment to prevent unauthorized file access and to only enable read-only mode if file uploads are not required.

Thank you for helping us keep the TFTP Server Daemon project secure!
