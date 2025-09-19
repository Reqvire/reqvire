# User Requirements

This is a test user requirements document for diagram generation.

## User Interface Requirements

```mermaid
graph TD;
  %% Graph styling
  classDef requirement fill:#f9d6d6,stroke:#f55f5f,stroke-width:1px;
  classDef verification fill:#d6f9d6,stroke:#5fd75f,stroke-width:1px;
  classDef externalLink fill:#d0e0ff,stroke:#3080ff,stroke-width:1px;
  classDef default fill:#f5f5f5,stroke:#333333,stroke-width:1px;

  435c8ddc18["UI Element 2"];
  click 435c8ddc18 "UserRequirements.md#ui-element-2";
  class 435c8ddc18 requirement;
  845c8ddc18["UI Element 1"];
  click 845c8ddc18 "UserRequirements.md#ui-element-1";
  class 845c8ddc18 requirement;
```

### Main User Interface Requirement

This is the main user interface requirement that defines overall UI behavior.

#### Metadata
  * type: user-requirement

### UI Element 1

First user interface requirement.

#### Relations
  * derivedFrom: [Main User Interface Requirement](#main-user-interface-requirement)
  * refinedBy: [SystemRequirements.md#system-element-1](SystemRequirements.md#system-element-1)

### UI Element 2

Second user interface requirement.

#### Relations
  * derivedFrom: [Main User Interface Requirement](#main-user-interface-requirement)
  * refinedBy: [SystemRequirements.md#system-element-2](SystemRequirements.md#system-element-2)

## Performance Requirements
```mermaid
graph TD;
  %% Graph styling
  classDef requirement fill:#f9d6d6,stroke:#f55f5f,stroke-width:1px;
  classDef verification fill:#d6f9d6,stroke:#5fd75f,stroke-width:1px;
  classDef externalLink fill:#d0e0ff,stroke:#3080ff,stroke-width:1px;
  classDef default fill:#f5f5f5,stroke:#333333,stroke-width:1px;

  8c5d893e6f["Performance Element 1"];
  click 8c5d893e6f "UserRequirements.md#performance-element-1";
  class 8c5d893e6f requirement;
  80d3ce63e4["Performance Element 2"];
  click 80d3ce63e4 "UserRequirements.md#performance-element-2";
  class 80d3ce63e4 requirement;
```


### Performance Element 1

First performance requirement.

#### Relations
  * derivedFrom: [UI Element 1](#ui-element-1)

### Performance Element 2

Second performance requirement.

#### Relations
  * derivedFrom: [UI Element 2](#ui-element-2)