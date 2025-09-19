# reqvire

**reqvire** is a lightweight, Git-native tool for system modeling, **requirements management**, and agile project specifications, designed to support real-world development with automation, **AI assistance**, and full **traceability**.

By using **semi-structured markdown** documents, reqvire bridges the gap between formal modeling and agile docs, unlocking AI-assisted requirements, architecture design and even code generation in a single, version-controlled workflow.


---
<sub>Figure 1: Example Diagram (click image to browse requirements)</sub>
[![Example Diagram](doc/diagram_1.png)](specifications/SpecificationsRequirements.md#specifications-requirements)

---

## Why reqvire?

- **Git-Native Workflow** – Requirements, specifications, and traceability artifacts live alongside code with full version control and collaboration via Git.
- **Agile & Human-Friendly** – Uses Markdown-based, semi-structured documents that are both readable and AI friendly.
- **Automation-Ready** – Supports traceability, impact analysis, validation, and documentation generation to reduce manual effort and enforce consistency.
- **Flexible Integration** – Seamlessly connects with GitHub, GitLab, CI/CD pipelines, and AI-driven tools to support continuous delivery and refinement.
- **MBSE-Inspired, Agile-Optimized** – Combines the rigor of Model-Based Systems Engineering with the speed and simplicity demanded by modern software teams.
- **Traceability & Impact Awareness** – Every change, decision, and dependency is trackable—enabling confident change management and system evolution.
- **Built for Developer Workflows** – Links directly to issues, pull requests, and test cases, keeping specifications aligned with real development activity.
- **Visual Modeling & Reporting** – Automatically generates diagrams, traceability matrices, and structured reports using tools like MermaidJS.
- **AI-Augmented Engineering** – Assists with requirement authoring, architecture analysis, validation, and code generation to boost both quality and velocity.

---

<sub>Figure 2: Example Change Impact Report (click image to read specifications)</sub>

[![Example Change Impact Report](doc/change_impact_report_pr.png)](specifications/SpecificationsRequirements.md#requirements-change-propagation)

---

## Get Started

For detailed information about **how to use reqvire** visit the [documentation](./doc/README.md).

To understand the use case of **reqvire**, see the [use case diagram](./specifications/Usecases.md).

### Installation

To set up reqvire, follow these steps:

#### Run install script (Linux and macOS)

Run the following command in your terminal:
```
curl -fsSL https://raw.githubusercontent.com/Reqvire/reqvire/main/scripts/install.sh | bash
```

#### From Source (Linux and macOS)

1. **Install Rust and Cargo**
   - Install Rust using rustup:
     ```bash
     curl -sSf https://sh.rustup.rs | sh
     ```
   - Follow the on-screen prompts to complete the installation
   - Verify the installation:
     ```bash
     rustc --version
     cargo --version
     ```

2. **Clone the reqvire Repository**
   ```bash
   git clone https://github.com/Reqvire/reqvire.git
   cd reqvire
   ```

3. **Build reqvire**
   ```bash
   cargo build --release
   ```

4. **Install the Binary** (optional)
   ```bash
   cargo install --path .
   ```
   This will install the binary to `~/.cargo/bin/reqvire`

#### Using Pre-built Binaries

1. **Download the Latest Release**
   - Visit the [Releases page](https://github.com/Reqvire/reqvire/releases) on GitHub
   - Download the appropriate binary for your platform:
     - Linux: `reqvire-linux-x86_64.tar.gz`
     - macOS (Apple Silicon): `reqvire-darwin-arm64.tar.gz`
     - macOS: `reqvire-darwin-x86_64.tar.gz`       

2. **Extract the Binary**
   ```bash
   tar -xzf reqvire-<platform>.tar.gz
   ```

3. **Move to a Directory in Your PATH**
   For most Linux and Intel-based macOS systems:
   ```bash
   sudo mv reqvire /usr/local/bin/
   ````
   
    For Apple Silicon (M1/M2) macOS:
    ```bash
    sudo mv reqvire /opt/homebrew/bin/   
    ```
    
4. **Verify the Installation**
   ```bash
   reqvire --version
   ```
    
### Configuration

Create `reqvire.yaml` in the root of the git repo with following minimal content:
```
  # Path to the specifications folder
  specifications_folder: "specifications"
  
  # Default output folder for exported html specifications
  output_folder: "html"
  
  # Additional external folders that contain system requirements and other files
  # Usually source folders relative to the repository root.
  # All markdown files in these folders are considered sytsem requirements (except those matching exclusion patterns)
  external_folders:
    - tests
    - core
    - cli
      
  # Glob patterns to exclude from requirements processing
  # These are patterns that shouldn't be considered requirements even if they're in specifications or external folders
  excluded_filename_patterns:
    - "Usecases.md"
    - "**/Logical*.md"
    - "**/Physical*.md"
    - "**/TODO.md"  
```

Create `output` directory.


    
## Contributing

We welcome contributions to **reqvire**! Whether you're improving the tooling, refining specifications, enhancing test cases, or shaping modeling conventions, your input is highly valued.

To maintain **consistency**, **traceability**, and **quality** in **reqvire**, we follow a strict **contribution discipline** that ensures the system model, tests, and verifications evolve alongside the code.

### Contribution Workflow

1. **Read the Contribution Guidelines**:
   - Begin by reviewing our [Contribution Guidelines](./doc/CONTRIBUTING.md) for details on processes, coding standards, and expectations.

2. **Fork the Repository**:
   - Create your own copy of the repository by forking it.

3. **Create a Feature Branch**:
   - Work on your changes in a dedicated branch:
     ```bash
     git checkout -b feature/your-feature-name
     ```

4. **Maintain Requirements, Tests, and Verifications**:
   - **Update or create requirements**:
     - For any **new code**, **feature**, or **change** in the codebase, always **create new requirements** or **update existing ones** to reflect those changes.
     - Requirements should be added in the appropriate **User Requirements** or **System Requirements** folders, following the Reqvire methodology.
     - Requirments must be approved before code can be implemented or existing functionality changed
   - **Add or update end-to-end (E2E) tests**:
     - If your changes introduce **new functionality** or modify existing behavior that is not covered by tests, ensure to **add E2E tests** in the `tests/` directory.
     - Tests must validate the **expected behavior** of your feature or change.
   - **Update verifications**:
     - For every **new requirement** or **E2E test**, create or update the corresponding **verification cases** in the `specifications/verificationCases/` directory.
     - Verifications ensure that **requirements** are linked to **tests**, and all features are properly validated.

5. **Test Your Changes**:
   - Run **reqvire commands** locally to ensure your requirements and verifications are consistent:
     ```bash
     reqvire format
     ```
   - Ensure that your code passes **all tests** and that **no existing functionality breaks**.

6. **Submit a Pull Request (PR)**:
   - Once your changes are ready, submit a pull request with a **clear and detailed description** of what you've implemented or fixed.
   - Include a summary of:
     - **Change impact report if requirements were updated**
     - **New or updated tests**
     - **New or updated verifications**

7. **Collaborate and Iterate**:
   - Engage with maintainers, respond to feedback, and collaborate to refine your PR until it's ready to be merged.

For more details, refer to the [Contributing Guide](./doc/CONTRIBUTING.md).


## Credits

**reqvire** is an open-source project created and maintained by [Ilija Ljubicic](https://github.com/ilijaljubicic). 

### Special Thanks:

- [Juanjo Andres](https://github.com/juanjoandres)  
  For valuable contributions to testing and in in shaping reqvire's direction, especially in the early phases of reqvire's development.

- [GrapheneDB](https://www.graphenedb.com/)  
  For **partial sponsorship** and for being the **first user** of reqvire. Their support helped shape the tool's early direction.

## License

Licensed under the [Apache 2.0 License](LICENSE).

