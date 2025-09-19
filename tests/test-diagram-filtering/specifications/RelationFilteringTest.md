# Relation Filtering Test Document

This document contains test data specifically designed to verify diagram relation filtering behavior.

## Parent Section
```mermaid
graph TD;
  %% Graph styling
  classDef requirement fill:#f9d6d6,stroke:#f55f5f,stroke-width:1px;
  classDef verification fill:#d6f9d6,stroke:#5fd75f,stroke-width:1px;
  classDef externalLink fill:#d0e0ff,stroke:#3080ff,stroke-width:1px;
  classDef default fill:#f5f5f5,stroke:#333333,stroke-width:1px;

  b924ef05355e5889["Parent Element"];
  class b924ef05355e5889 requirement;
  click b924ef05355e5889 "tests/test-diagram-filtering/specifications/RelationFilteringTest.md#parent-element";
  285e1cb6bfc54ab5["Child Element"];
  class 285e1cb6bfc54ab5 requirement;
  click 285e1cb6bfc54ab5 "tests/test-diagram-filtering/specifications/RelationFilteringTest.md#child-element";
  b924ef05355e5889 --o|contains| 285e1cb6bfc54ab5;
  9671c9cefa336764["Derived Child"];
  class 9671c9cefa336764 requirement;
  click 9671c9cefa336764 "tests/test-diagram-filtering/specifications/RelationFilteringTest.md#derived-child";
  b924ef05355e5889 -.->|deriveReqT| 9671c9cefa336764;
```
### Parent Element

This is a parent element that should be included in child section diagrams.

#### Metadata
  * type: user-requirement

#### Relations
  * contain: [Child Element](#child-element)
  * derive: [Derived Child](#derived-child)

## Child Section
```mermaid
graph TD;
  %% Graph styling
  classDef requirement fill:#f9d6d6,stroke:#f55f5f,stroke-width:1px;
  classDef verification fill:#d6f9d6,stroke:#5fd75f,stroke-width:1px;
  classDef externalLink fill:#d0e0ff,stroke:#3080ff,stroke-width:1px;
  classDef default fill:#f5f5f5,stroke:#333333,stroke-width:1px;

  9671c9cefa336764["Derived Child"];
  class 9671c9cefa336764 requirement;
  click 9671c9cefa336764 "tests/test-diagram-filtering/specifications/RelationFilteringTest.md#derived-child";
  1582ff44c7eea4c7["Refined Element"];
  class 1582ff44c7eea4c7 requirement;
  click 1582ff44c7eea4c7 "tests/test-diagram-filtering/specifications/RelationFilteringTest.md#refined-element";
  9671c9cefa336764 -->|refines| 1582ff44c7eea4c7;
  285e1cb6bfc54ab5["Child Element"];
  class 285e1cb6bfc54ab5 requirement;
  click 285e1cb6bfc54ab5 "tests/test-diagram-filtering/specifications/RelationFilteringTest.md#child-element";
  ac87d933df745cc6["implementation.rs"];
  class ac87d933df745cc6 default;
  click ac87d933df745cc6 "tests/test-diagram-filtering/specifications/implementation.rs";
  285e1cb6bfc54ab5 -->|satisfies| ac87d933df745cc6;
  3358e23f1ddd1645["Test Verification"];
  class 3358e23f1ddd1645 verification;
  click 3358e23f1ddd1645 "tests/test-diagram-filtering/specifications/RelationFilteringTest.md#test-verification";
  285e1cb6bfc54ab5 -.->|verifies| 3358e23f1ddd1645;
  3ba472df84519f34["refined_impl.rs"];
  class 3ba472df84519f34 default;
  click 3ba472df84519f34 "tests/test-diagram-filtering/specifications/refined_impl.rs";
  1582ff44c7eea4c7 -->|satisfies| 3ba472df84519f34;
  b924ef05355e5889["Parent Element"];
  class b924ef05355e5889 requirement;
  click b924ef05355e5889 "tests/test-diagram-filtering/specifications/RelationFilteringTest.md#parent-element";
  b924ef05355e5889 --o|contains| 285e1cb6bfc54ab5;
  b924ef05355e5889 -.->|deriveReqT| 9671c9cefa336764;
```
### Child Element

This element has a parent in a different section to test hierarchy inclusion.

#### Relations
  * containedBy: [Parent Element](#parent-element)
  * satisfiedBy: [implementation.rs](implementation.rs)
  * verifiedBy: [Test Verification](#test-verification)

### Derived Child

This element is derived from the parent to test derivation relationships.

#### Relations
  * derivedFrom: [Parent Element](#parent-element)
  * refinedBy: [Refined Element](#refined-element)

### Test Verification

This verification element tests the child element.

#### Metadata
  * type: verification

#### Relations
  * verify: [Child Element](#child-element)

### Refined Element

This element refines the derived child with more details.

#### Relations
  * refine: [Derived Child](#derived-child)
  * satisfiedBy: [refined_impl.rs](refined_impl.rs)