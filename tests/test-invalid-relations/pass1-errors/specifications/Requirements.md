# Pass 1 Error Test Requirements

This document contains requirements with Pass 1 validation errors (parsing/format issues).

## Requirements

### Valid Requirement

This is a valid requirement.

#### Metadata
  * type: user-requirement

#### Relations
  * satisfiedBy: [ValidImplementation.txt](ValidImplementation.txt)

---

### Valid Requirement

Duplicate element name - should trigger duplicate element error.

#### Metadata
  * type: user-requirement

#### Relations
  * derivedFrom: [Valid Requirement](#valid-requirement)

---

### Requirement with Invalid Metadata Format

This requirement has invalid metadata format.

#### Metadata
This is not ok - invalid format.

#### Relations
  * derivedFrom: [Valid Requirement](#valid-requirement)

---

### Requirement with Invalid Relation Format

This requirement has invalid relations format.

#### Relations
  * derivedFrom: [Valid Requirement](#valid-requirement)
This is not ok - invalid format.

---

### Requirement with Invalid Relation Type

This requirement has an invalid relation type with a typo.

#### Metadata
  * type: user-requirement

#### Relations
  * derivedFrom: [Valid Requirement](#valid-requirement)
  * satisfiedBBy: [ValidImplementation.txt](ValidImplementation.txt)

---

### Requirement with Duplicate Subsection

This requirement has duplicate subsection.

#### Metadata
  * type: user-requirement

#### Relations
  * derivedFrom: [Valid Requirement](#valid-requirement)

#### Relations
  * satisfiedBy: [ValidImplementation.txt](ValidImplementation.txt)

---