# v0.1 compatibility declaration

| Environment | Declared gate | Scope |
| --- | --- | --- |
| Ubuntu GitHub Actions, Linux x86_64 | fmt, Clippy, locked workspace tests, 100 broken/fixed pairs, external target negative tests, independent Python/Rust evidence and README demo | Full local acceptance |
| macOS GitHub Actions | locked workspace build, portable workspace tests excluding 100-pair engine integration, v1 schema validation | Portable build and integration coverage |
| Windows GitHub Actions | locked workspace build, portable workspace tests excluding 100-pair engine integration, v1 schema validation | Portable build and integration coverage |
| Linux x86_64 clean checkout | locked release build; CLI version; schema v2 rejection; broken/fixed release-binary demos; independent report; local tarball + SHA-256 | Candidate packaging/preflight |
| Other OS/arch | No CI proof | UNVERIFIED |

Windows directory sync is intentionally not attempted as POSIX File::open(directory). All evidence **files** are individually synced; Linux/macOS additionally sync the directory where supported. This is not a guarantee of durability under arbitrary power loss.

Portable workspace CI does not run the 100-pair Linux stress matrix on Windows/macOS. No remote target, networked database, TLS, containers, root privileges, or cloud account is required. Linux tarball is a local candidate, not an official published release; source is Apache-2.0 licensed, but complete third-party license/notice review and official-release approval remain open.
