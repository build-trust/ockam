// This project follows Conventional Commits v1.0.0-beta.2
// The commit conventions are enforced using commitlint and this file configures commitlint
//
// https://www.conventionalcommits.org/en/v1.0.0-beta.2/#summary
// https://www.conventionalcommits.org/en/v1.0.0-beta.2/#specification
module.exports = {
	// This imports the conventional commit rules
	// https://github.com/marionebl/commitlint/blob/v7.1.2/%40commitlint/config-conventional/index.js
	extends: ["@commitlint/config-conventional"],

	// Allowed, type values:
	// build: Changes that affect the build system or external dependencies
	// ci: Changes to our CI configuration files and scripts
	// docs: Documentation only changes
	// feat: A new feature
	// fix: A bug fix
	// perf: A code change that improves performance
	// refactor: A code change that neither fixes a bug nor adds a feature
	// style: Changes that do not affect the meaning of the code (white-space, formatting, missing semi-colons, etc)
	// test: Adding missing tests or correcting existing tests

	// Addtional rules may be defined:
	// https://marionebl.github.io/commitlint/#/reference-rules
};
