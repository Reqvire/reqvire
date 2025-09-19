## System Overview
Reqvire is an AI-driven framework for system modeling and requirements management that integrates with Git workflows. It processes semi structured Markdown documents containing system requirements, manages relationships between elements, validates model consistency, and generates documentation and visualizations.


## Building and Running Reqvire
- Build: `cargo build`
- Run with HTML output: `cargo run -- specifications output --html`
- Run specific commands: `cargo run -- format` or `./target/debug/reqvire format`
- Test: `cargo test`
- Test specific: `cargo test <test_name>`
- Lint: `cargo clippy -- -D warnings`
- Format: `cargo fmt`

## Core Commands
- Generate HTML documentation: `./target/debug/reqvire html`
- Format requirements (preview only): `./target/debug/reqvire format --dry-run`
- Apply formatting: `./target/debug/reqvire format`
- Generate diagrams: `./target/debug/reqvire generate-diagrams`
- Generate model summary: `./target/debug/reqvire model-summary --json > /tmp/model-summary.json`

| Filter Type | Command Example | Description |
|-------------|----------------|-------------|
| File path | `--filter-file="src/**/*Reqs.md"` | Filter elements by file glob pattern |
| Name regex | `--filter-name=".*safety.*"` | Filter elements by name using regex |
| Section | `--filter-section="System*"` | Filter elements by section glob pattern |
| Type | `--filter-type="system-requirement"` | Filter by exact element type |
| Content | `--filter-content="MUST"` | Filter elements containing specific text |
| Not verified | `--filter-is-not-verified` | Show only unverified requirements |
| Not satisfied | `--filter-is-not-satisfied` | Show only unsatisfied requirements |

Examples:
- Human-readable output: `./target/debug/reqvire model-summary`
- JSON output with filter: `./target/debug/reqvire model-summary --json --filter-type="system-requirement"`
- Multiple filters: `./target/debug/reqvire model-summary --filter-file="src/**/*Reqs.md" --filter-section="System*" --filter-is-not-verified`
- Show change impact: `./target/debug/reqvire change-impact --git-commit=HEAD~1 --json > /tmp/impact.json`
- Generate traceability matrix: `./target/debug/reqvire traces --json > /tmp/traces.json`

## Important Files (Auto-Imported)                                                                                                                                                                      │ │

@specifications/SpecificationsRequirements.md                                                                                                                                                           │ │


## Key File Locations
- Core specifications structure: [SpecificationsRequirements.md](https://github.com/ilijaljubicic/Reqvire/blob/main/specifications/SpecificationsRequirements.md)
- User requirements: [UserRequirements.md](https://github.com/ilijaljubicic/Reqvire/blob/main/specifications/UserRequirements.md)
- Mission requirements: [MissionRequirements.md](https://github.com/ilijaljubicic/Reqvire/blob/main/specifications/MissionRequirements.md)
- System requirements: [SystemRequirements/Requirements.md](https://github.com/ilijaljubicic/Reqvire/blob/main/specifications/SystemRequirements/Requirements.md)
- Verification specifications: [Verifications folder](https://github.com/ilijaljubicic/Reqvire/tree/main/specifications/Verifications)
- Relationship types: [Relation Types and Behaviors](https://github.com/ilijaljubicic/Reqvire/blob/main/specifications/SpecificationsRequirements.md#relation-types-and-behaviors)
- Architecture documentation: [LogicalArchitecture.md](https://github.com/ilijaljubicic/Reqvire/blob/main/specifications/LogicalArchitecture.md) and [PhysicalArchitecture.md](https://github.com/ilijaljubicic/Reqvire/blob/main/specifications/PhysicalArchitecture.md)


## Core Components
- **ModelManager**: Central coordinator for processing requirements
- **ElementRegistry**: Tracks and retrieves elements by identifier
- **Element**: Represents MBSE model elements (requirements, verifications)
- **Relation**: Represents connections between elements
- **Parser**: Processes Markdown to extract elements and relations
- **Linting**: Validates and fixes markdown formatting issues
- **ChangeImpact**: Analyzes how changes propagate through the model
- **MatrixGenerator**: Creates traceability matrices
- **DiagramGenerator**: Creates Mermaid diagrams from relationships

## Important

Use the `/tmp` directory to store JSON outputs for further analysis.

## Development Guidelines
- Use Result with custom error types for error handling
- Document all public APIs with rustdoc
- Organize code in modules by functionality
- Follow the requirements and architecture defined in specifications/
- link code and implementations to requirements via satisfiedBy on requirement level

## Code Style Guidelines
- Follow Rust standard naming conventions
- snake_case for functions, variables, modules
- CamelCase for types, traits, enums
- SCREAMING_SNAKE_CASE for constants
- Rust 2021 edition features
- Error handling: use Result with custom error types
- Document all public APIs with rustdoc
- Organize code in modules by functionality
- Prefer strong typing over string manipulation
- Follow the requirements, logical and physical architecture defined in specifications/
- specifically read and understand Design Specifications in specifications/DesignSpecifications/
