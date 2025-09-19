# Pass 2 Error Test Requirements

This document contains requirements with Pass 2 validation errors (relation validation issues).

## Requirements

### Valid User Requirement

This is a valid user requirement that serves as a parent.

#### Metadata
  * type: user-requirement

#### Relations
  * satisfiedBy: [ValidImplementation.txt](ValidImplementation.txt)

---

### System Requirement with Missing Parent Relation

This system requirement is missing parent relation.

#### Metadata
  * type: system-requirement

---

### Requirement with Missing Target

This requirement has a relation to a non-existent element.

#### Metadata
  * type: system-requirement

#### Relations
  * containedBy: [Valid User Requirement](#valid-user-requirement)
  * satisfiedBy: [NonExistentElement.md#missing-element](NonExistentElement.md#missing-element)

---

### Requirement with Incompatible Element

This requirement has incompatible element in relation - satisfiedBy cannot point to other requirement.

#### Metadata
  * type: system-requirement

#### Relations
  * containedBy: [Valid User Requirement](#valid-user-requirement)
  * satisfiedBy: [Valid User Requirement](#valid-user-requirement)

---

### Requirement with Circular Dependencies A

This requirement has circular dependency.

#### Metadata
  * type: system-requirement

#### Relations
  * containedBy: [Valid User Requirement](#valid-user-requirement)
  * derivedFrom: [Requirement with Circular Dependencies C](#requirement-with-circular-dependencies-c)

---

### Requirement with Circular Dependencies B

This requirement has circular dependency.

#### Metadata
  * type: system-requirement

#### Relations
  * containedBy: [Valid User Requirement](#valid-user-requirement)
  * derivedFrom: [Requirement with Circular Dependencies A](#requirement-with-circular-dependencies-a)

---

### Requirement with Circular Dependencies C

This requirement has circular dependency.

#### Metadata
  * type: system-requirement

#### Relations
  * containedBy: [Valid User Requirement](#valid-user-requirement)
  * derivedFrom: [Requirement with Circular Dependencies B](#requirement-with-circular-dependencies-b)

---

### Verification with Missing SatisfiedBy Target

This verification has a satisfiedBy relation pointing to a non-existing file.

#### Metadata
  * type: verification

#### Relations
  * verify: [Valid User Requirement](#valid-user-requirement)
  * satisfiedBy: [non-existing-test-script.sh](non-existing-test-script.sh)

---

### Requirement with Missing VerifiedBy Target

This requirement has a verifiedBy relation pointing to a non-existing verification element.

#### Metadata
  * type: system-requirement

#### Relations
  * containedBy: [Valid User Requirement](#valid-user-requirement)
  * verifiedBy: [NonExistentVerification.md#missing-verification](NonExistentVerification.md#missing-verification)

---

### Valid Verification with Correct SatisfiedBy

This verification correctly uses satisfiedBy pointing to an existing test script.

#### Metadata
  * type: verification

#### Relations
  * verify: [Valid User Requirement](#valid-user-requirement)
  * satisfiedBy: [test.sh](test.sh)

---

### Requirement SatisfiedBy Requirement Invalid

This requirement incorrectly uses satisfiedBy pointing to another requirement (should fail).

#### Metadata
  * type: system-requirement

#### Relations
  * containedBy: [Valid User Requirement](#valid-user-requirement)
  * satisfiedBy: [Valid User Requirement](#valid-user-requirement)

---

### Verification SatisfiedBy Verification Invalid

This verification incorrectly uses satisfiedBy pointing to another verification (should fail).

#### Metadata
  * type: verification

#### Relations
  * verify: [Valid User Requirement](#valid-user-requirement)
  * satisfiedBy: [Valid Verification with Correct SatisfiedBy](#valid-verification-with-correct-satisfiedby)

---

### Requirement with Missing SatisfiedBy File

This requirement has a satisfiedBy relation pointing to a non-existing file.

#### Metadata
  * type: system-requirement

#### Relations
  * containedBy: [Valid User Requirement](#valid-user-requirement)
  * satisfiedBy: [non-existing-implementation.py](non-existing-implementation.py)

---