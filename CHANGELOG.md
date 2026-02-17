## [0.6.0] - 2026-02-17

### 🚀 Features

- Add ImageTag enum and use it in CreateDeploymentOptions ([#76](https://github.com/mongodb/atlas-local-lib/pull/76))
## [0.5.0] - 2026-02-13

### 🚀 Features

- Add voyage api key and move preview to mod_version enum ([#73](https://github.com/mongodb/atlas-local-lib/pull/73))
## [0.4.2] - 2026-02-12

### 🚀 Features

- Add use_preview_tag option to create deployment options ([#72](https://github.com/mongodb/atlas-local-lib/pull/72))
## [0.4.0] - 2026-01-12

### 🚀 Features

- Make it possible to follow createDeployment progress ([#63](https://github.com/mongodb/atlas-local-lib/pull/63))
## [0.3.0] - 2026-01-05

### 🚀 Features

- Implement start/stop/pause/unpause/wait for healthy deployment ([#54](https://github.com/mongodb/atlas-local-lib/pull/54))
## [0.2.0] - 2025-12-19

### 🚜 Refactor

- Update get_logs implementation to return logs as a vector and remove LogOutputStream ([#53](https://github.com/mongodb/atlas-local-lib/pull/53))
## [0.1.1] - 2025-12-18

### 🚀 Features

- Added client.get_logs ([#51](https://github.com/mongodb/atlas-local-lib/pull/51))

### ⚙️ Miscellaneous Tasks

- Fix release + make publish --dry-run part of CI checks ([#52](https://github.com/mongodb/atlas-local-lib/pull/52))
## [0.1.0] - 2025-12-16

### 🚀 Features

- Implement list_deployments ([#3](https://github.com/mongodb/atlas-local-lib/pull/3))
- Implement delete_deployement - MCP-145 ([#5](https://github.com/mongodb/atlas-local-lib/pull/5))
- Added port and status to deployment ([#7](https://github.com/mongodb/atlas-local-lib/pull/7))
- Implement create_deployment ([#6](https://github.com/mongodb/atlas-local-lib/pull/6))
- Add e2e job to CI that runs Docker-in-Docker ([#10](https://github.com/mongodb/atlas-local-lib/pull/10))
- Add end to end tests ([#9](https://github.com/mongodb/atlas-local-lib/pull/9))
- Add wait_until_healthy option to CreateDeploymentOptions ([#12](https://github.com/mongodb/atlas-local-lib/pull/12))
- Change create_deployment to return deployment ([#15](https://github.com/mongodb/atlas-local-lib/pull/15))
- Add Coveralls  ([#16](https://github.com/mongodb/atlas-local-lib/pull/16))
- Implements get_connection_string ([#14](https://github.com/mongodb/atlas-local-lib/pull/14))
- Implement get deployment id ([#20](https://github.com/mongodb/atlas-local-lib/pull/20))
- [**breaking**] Git add emoved verify option and switch to quay.io ([#21](https://github.com/mongodb/atlas-local-lib/pull/21))
- Get deployment id without networking, fixes library inside of docker ([#23](https://github.com/mongodb/atlas-local-lib/pull/23))
- Add support for loading sample data ([#44](https://github.com/mongodb/atlas-local-lib/pull/44))
- Integrate serde for serialization and deserialization ([#48](https://github.com/mongodb/atlas-local-lib/pull/48))

### 🐛 Bug Fixes

- Remove dbusename and dbpassword inputs from get_connection_string ([#27](https://github.com/mongodb/atlas-local-lib/pull/27))
- Add support multiple port bindings when they're ipv4 and ipv6 equivalents ([#37](https://github.com/mongodb/atlas-local-lib/pull/37))

### ⚙️ Miscellaneous Tasks

- Set up scaffolding for project
- Set up github actions ([#1](https://github.com/mongodb/atlas-local-lib/pull/1))
- Add CODEOWNERS file ([#2](https://github.com/mongodb/atlas-local-lib/pull/2))
- Cache cargo-deny and cargo-audit to improve CI/CD speed ([#4](https://github.com/mongodb/atlas-local-lib/pull/4))
- Enforce conventional commit style ([#8](https://github.com/mongodb/atlas-local-lib/pull/8))
- Split code and introduce traits to improve testability ([#11](https://github.com/mongodb/atlas-local-lib/pull/11))
- Improved unit test coverage ([#13](https://github.com/mongodb/atlas-local-lib/pull/13))
- *(readme)* Added badges to README.md ([#19](https://github.com/mongodb/atlas-local-lib/pull/19))
- Makes MongoDbClient  trait thread-safe ([#22](https://github.com/mongodb/atlas-local-lib/pull/22))
- Dissalow use of unwrap/expect/panic in codebase ([#24](https://github.com/mongodb/atlas-local-lib/pull/24))
- Added unused packages check ([#25](https://github.com/mongodb/atlas-local-lib/pull/25))
- Verify third-party licenses on PR and change to plaintext format from html third-party licenses ([#26](https://github.com/mongodb/atlas-local-lib/pull/26))
- Added dependabot for cargo and github actions ([#28](https://github.com/mongodb/atlas-local-lib/pull/28))
- Fix merge permission + also allow merging github actions updates ([#33](https://github.com/mongodb/atlas-local-lib/pull/33))
- Change merge strategy to --squash for dependabot prs ([#34](https://github.com/mongodb/atlas-local-lib/pull/34))
- Re-generate third party licenses on dependabot PR ([#35](https://github.com/mongodb/atlas-local-lib/pull/35))
- Fix auto-merge for github action dependabot updates ([#36](https://github.com/mongodb/atlas-local-lib/pull/36))
- Improve boolean handling ([#45](https://github.com/mongodb/atlas-local-lib/pull/45))
- Set up release github action ([#49](https://github.com/mongodb/atlas-local-lib/pull/49))
- Fix release process ([#50](https://github.com/mongodb/atlas-local-lib/pull/50))
