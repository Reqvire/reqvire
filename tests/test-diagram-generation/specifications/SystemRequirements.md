# System Requirements

This is a test system requirements document for diagram generation.

## Functional Requirements
```mermaid
graph TD;
  %% Graph styling
  classDef requirement fill:#f9d6d6,stroke:#f55f5f,stroke-width:1px;
  classDef verification fill:#d6f9d6,stroke:#5fd75f,stroke-width:1px;
  classDef externalLink fill:#d0e0ff,stroke:#3080ff,stroke-width:1px;
  classDef default fill:#f5f5f5,stroke:#333333,stroke-width:1px;

  49bcee0192["System Element 2"];
  click 49bcee0192 "SystemRequirements.md#system-element-2";
  class 49bcee0192 requirement;
  ccfb36da70["System Element 1"];
  click ccfb36da70 "SystemRequirements.md#system-element-1";
  class ccfb36da70 requirement;
```


### System Element 1

First system requirement.

#### Relations
  * refine: [UserRequirements.md#ui-element-1](UserRequirements.md#ui-element-1)

### System Element 2

Second system requirement.

#### Relations
  * refine: [UserRequirements.md#ui-element-2](UserRequirements.md#ui-element-2)

## Security Requirements
```mermaid
graph TD;
  %% Graph styling
  classDef requirement fill:#f9d6d6,stroke:#f55f5f,stroke-width:1px;
  classDef verification fill:#d6f9d6,stroke:#5fd75f,stroke-width:1px;
  classDef externalLink fill:#d0e0ff,stroke:#3080ff,stroke-width:1px;
  classDef default fill:#f5f5f5,stroke:#333333,stroke-width:1px;

  c67ea528b4["System Element 3"];
  click c67ea528b4 "SystemRequirements.md#system-element-3";
  class c67ea528b4 requirement;
  8bf4ab4807["System Verification Test 1"];
  class 8bf4ab4807 default;
  click 8bf4ab4807 "SystemRequirements.md#system-verification-test-1";
  8bf4ab4807 -->|verifies| c67ea528b4;
  43405c4fd6["System Element 4"];
  click 43405c4fd6 "SystemRequirements.md#system-element-4";
  class 43405c4fd6 requirement;
  4af4ab4807["System Verification Test 2"];
  class 4af4ab4807 default;
  click 4af4ab4807 "SystemRequirements.md#system-verification-test-2";
  4af4ab4807 -->|verifies| 43405c4fd6;
```


### System Element 3

System security requirement.

#### Relations
  * derivedFrom: [System Element 1](#system-element-1)
  * verifiedBy: [VerificationTests.md#system-verification-test-1](VerificationTests.md#system-verification-test-1)

### System Element 4

Another system security requirement.

#### Relations
  * derivedFrom: [System Element 2](#system-element-2)
  * verifiedBy: [VerificationTests.md#system-verification-test-2](VerificationTests.md#system-verification-test-2)